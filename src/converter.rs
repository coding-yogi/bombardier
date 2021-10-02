
use reqwest::Request as Reqwest;
use reqwest::{
    RequestBuilder,
    header::{
        HeaderMap, 
        HeaderName, 
        HeaderValue
    },
    Method,
    multipart::{Form,Part}
};
use tokio::fs;

use std::{
    str::FromStr,
    collections::HashMap,
    error::Error as StdError
};

use crate::model::{Request, Body};
use crate::protocol::http::HttpClient;

pub async fn convert_request(http_client: &HttpClient, request: &Request) -> Result<Reqwest, Box<dyn StdError + Send + Sync>> {
    //Method
    let method = Method::from_str(&request.method)?;

    //Headers
    let headers = match get_header_map_from_request(request) {
        Ok(headers) => headers,
        Err(err) => return Err(err)
    };

     //Create builder
     let mut builder = http_client
        .get_client()
        .request(method, request.url.to_owned()).headers(headers);

    //Body
    let body = &request.body;

    if !body.raw.is_empty()  {
        builder = builder.body(body.raw.to_owned());
    } else if !body.formdata.is_empty() {
        builder = add_multipart_form_data(builder, body).await?;
    } else if !body.urlencoded.is_empty() {
        builder = add_url_encoded_data(builder, body);
    } 

    Ok(builder.build()?)
}

fn get_header_map_from_request(request: &Request) 
-> Result<HeaderMap, Box<dyn std::error::Error + Send + Sync>> {
    let mut headers = HeaderMap::new();
    for header in &request.headers {
        headers.insert(HeaderName::from_str(header.0.as_str().unwrap())?, 
        HeaderValue::from_str(header.1.as_str().unwrap())?);
    }

    Ok(headers)
}

async fn add_multipart_form_data(builder: RequestBuilder, body: &Body) 
-> Result<RequestBuilder, Box<dyn std::error::Error + Send + Sync>> {
    let mut form = Form::new();

    for data in &body.formdata {
        let param_key = data.0.as_str().unwrap();
        let param_value = data.1.as_str().unwrap();

        if param_key.to_lowercase().starts_with("_file") {
            let contents = fs::read_to_string(param_value).await?;
            let file_name = get_file_name(param_value)?;
            let part = Part::stream(contents).file_name(file_name)
                                .mime_str("application/octet-stream").unwrap();
            form = form.part("", part);
        } else {
            form = form.text(param_key.to_owned(), param_value.to_owned());
        }
    }

    Ok(builder.multipart(form))
}

fn get_file_name(path: &str) -> Result<String, tokio::io::Error> {
    let iter = path.split('/');
    Ok(iter.last().unwrap().to_string())
}

fn add_url_encoded_data(builder: RequestBuilder, body: &Body) -> RequestBuilder {
    let mut params = HashMap::with_capacity(body.urlencoded.len());

    body.urlencoded.iter().for_each(|(k,v)| {
        params.insert(k.as_str().unwrap().to_owned(), v.as_str().unwrap().to_owned());
    });

    builder.form(&params)
}