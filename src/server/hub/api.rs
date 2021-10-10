use futures::TryStreamExt;
use log::{error, info};
use serde::{Serialize, Deserialize};
use warp::{
    Buf, 
    Rejection, 
    Reply, 
    http::StatusCode, 
    multipart::{
        FormData, 
        Part
    }, 
    reply::{
        self, 
        Json, 
        WithStatus
    }
};

use std::{
    sync::Arc,
    str::from_utf8,
};

use crate::{
    bombardier::Bombardier, 
    parse::parser, 
    server::servers
};

enum ContentType {
    Yml
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Yml => "text/yaml"
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ErrorResponse {
    code: u16,
    description: String
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct SuccessResponse {
    message: String
}

impl ErrorResponse {
    fn new(code: u16, description: &str) -> Self {
        ErrorResponse{ 
            code, 
            description: String::from(description) 
        }
    }

    fn get_warp_reply(&self) -> Result<WithStatus<Json>, Rejection> {
        Ok(reply::with_status(reply::json(self), 
        StatusCode::from_u16(self.code).unwrap()))
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct NodesResponse {
    available_nodes: usize,
    bombarding_nodes: usize,
}

impl NodesResponse {
    fn new(available_nodes: usize, bombarding_nodes: usize) -> Self {
        NodesResponse{ 
            available_nodes ,
            bombarding_nodes
        }
    }

    fn get_warp_reply(&self) -> Result<WithStatus<Json>, Rejection> {
        Ok(reply::with_status(reply::json(self), 
        StatusCode::OK))
    }
}

pub async fn start(ctx: Arc<servers::Context>, form_data: FormData, ) -> Result<impl Reply, Rejection> {   
    //Check if nodes are available
    info!("Checking whether nodes are available for execution");
    let total_nodes = ctx.get_total_nodes().await;

    if total_nodes == 0 {
        return ErrorResponse::new(500, "No nodes available for execution").get_warp_reply();
    }

    //check if execution in progress
    info!("Checking if any execution is in progress");
    if ctx.get_currently_bombarding_nodes().await > 0{
        return ErrorResponse::new(500, "Bombarding in progress").get_warp_reply();
    }

    let parts: Vec<Part> = form_data.try_collect().await.map_err(|e| {
        eprintln!("form error: {}", e);
        warp::reject::reject()
        //Ok(single_error_response(StatusCode::BAD_REQUEST, &e.to_string()))
    })?;

    let mut errors = vec![];

    let mut config_present = false;
    let mut scenario_present = false;

    let mut config_content = String::new();
    let mut scenarios_content = String::new();
    let mut environments_content = String::new();
    let mut data_file_path = String::new();

    //Check if all files received
    for mut p in parts {
        if p.filename().is_some() {
            info!("Reading {} param", p.name());
            match p.name() {
                "config" => {
                    config_present = true;
                    match validate_content_type(&p, ContentType::Yml) {
                        Some(error) =>  errors.push(error),
                        None => config_content = get_stream(p).await
                    };
                },
                "scenarios" => {
                    scenario_present = true;
                    match validate_content_type(&p, ContentType::Yml) {
                        Some(error) =>  errors.push(error),
                        None => scenarios_content = get_stream(p).await
                    };
                },
                "environment" => {
                    match validate_content_type(&p, ContentType::Yml) {
                        Some(error) =>  errors.push(error),
                        None => environments_content = get_stream(p).await
                    };
                },
                _ => {
                    error!("Invalid parameter name {}", p.name());
                }
            }
        } else if p.name() == "data" {
            data_file_path = match p.data().await {
                Some(path) => match path {
                    Ok(mut b) => {
                        let mut v = Vec::new();
                        while b.has_remaining() {
                            v.push(b.get_u8());
                        };

                        from_utf8(&v).unwrap().to_owned()
                    },
                    Err(err) => {
                        error!("Error occured while reading the data: {}", err.to_string());
                        errors.push(ErrorResponse::new(400, "Error occured while reading the data"));
                        String::with_capacity(0)
                    }
                },
                None => String::with_capacity(0)
            };
        }
    }

    //Check all mandatory files are present
    if !config_present || !scenario_present {
        error!("config and scenario file parameters are mandatory");
        errors.push(ErrorResponse::new(400, "config and scenarios file parameters are mandatory"));
    }

    //Check error vector
    if !errors.is_empty() {
        return Ok(reply::with_status(reply::json(&errors), StatusCode::BAD_REQUEST));
    }

    //Parse config
    info!("Parsing config file content");
    let mut config = match parser::parse_config(&config_content) {
        Ok(config) => config,
        Err(err) => return ErrorResponse::new(400, &err.to_string()).get_warp_reply()
    };

    //set distributed to true
    config.distributed = true;
    config.data_file = data_file_path;

    //Prepare bombardier message
    info!("Preparing bombardier message");
    let bombardier = 
    match Bombardier::new(config, environments_content, scenarios_content) {
        Ok(bombardier) => bombardier,
        Err(err) => return ErrorResponse::new(400, &err.to_string()).get_warp_reply()
    };
    
    //Send the bombard message via transmitter
    {
        let trasmitter_map_mg = ctx.transmitters_map.lock().await;
        trasmitter_map_mg.iter()
            .for_each(|entry| {
                match entry.1.send(bombardier.clone()) {
                    Ok(_) => (),
                    Err(err) => {
                        let error = "Error occured while sending message to node";
                        error!("{} {} : {}", error, entry.0, &err);
                    }
                };
            })
    }

    Ok(reply::with_status(
        reply::json(&SuccessResponse{
            message: String::from("execution started successfully")
        }), StatusCode::CREATED))
}

pub async fn stop(_: Arc<servers::Context>) -> Result<impl Reply, Rejection> {
    Ok(StatusCode::OK)
}

pub async fn nodes(ctx: Arc<servers::Context>) -> Result<impl Reply, Rejection> {
    NodesResponse::new(ctx.get_total_nodes().await, 
        ctx.get_currently_bombarding_nodes().await).get_warp_reply()
}

async fn get_stream(p: Part) -> String {
    let v= p.stream()
        .try_fold(Vec::new(), |mut v, mut data| {
            while data.has_remaining() {
                v.push(data.get_u8());
            }
            async move { Ok(v) }
        })
        .await.unwrap();

    from_utf8(&v).unwrap().to_owned()
}

fn validate_content_type(p: &Part, expected_type: ContentType) -> Option<ErrorResponse> {
    let actual_content_type = p.content_type().unwrap();
    let expected_content_type = expected_type.as_str();

    if actual_content_type != expected_content_type {
        let error_description = format!("{} param should be of type {}", p.filename().unwrap(), expected_content_type);
        error!("{}", error_description);
        return Some(ErrorResponse::new(400, &error_description))
    }

    None
}