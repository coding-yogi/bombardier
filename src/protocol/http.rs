use log::{info, warn};
use reqwest::{
    Client, 
    Response, 
    RequestBuilder,
    header::{
        HeaderMap, 
        HeaderName, 
        HeaderValue},
    Certificate, 
    Identity, 
    Method,
    multipart::{Form,Part}
};

use std::{
    collections::HashMap,
    error::Error,
    str::FromStr,
    time,
};

use crate::{cmd, file, model::{Request, Body}};

pub fn get_default_sync_client() -> Client {
    Client::builder()
        .user_agent("bombardier")
        .build()
        .expect("Unable to create default sync client")
} 

async fn get_certificate(path: &str)  -> Result<Certificate, Box<dyn Error + Send + Sync>> {
    let cert = file::read_file(path).await?;
    if path.to_lowercase().ends_with(cmd::DER_EXT) {
        return Ok(Certificate::from_der(&cert)?)
    } else if path.to_lowercase().ends_with(cmd::PEM_EXT) {
        return Ok(Certificate::from_pem(&cert)?)
    }

    Err("Certificate should be in .pem or .der format".into())
}

async fn get_identity(path: &str, password: &str) -> Result<Identity, Box<dyn Error + Send + Sync>> {
    let ks = file::read_file(path).await?;
    if path.to_lowercase().ends_with(cmd::P12_EXT) || path.to_lowercase().ends_with(cmd::PFX_EXT) {
        return Ok(Identity::from_pkcs12_der(&ks, password)?)
    }

    Err("Keystore should be in .p12 or .pfx format".into())
}

fn is_method_valid(method_name: &str) -> bool {
    let method = method_name.to_uppercase();
    !(method != "GET" && method != "POST" && method != "PUT" && method != "PATCH" && method != "OPTIONS")
}

pub async fn get_async_client(config: &cmd::ExecConfig)  -> Result<Client, Box<dyn Error + Send + Sync>> {
    let mut client_builder = Client::builder()
        .user_agent("bombardier")
        .use_native_tls();

    if config.handle_cookies {
        info!("Enabling cookie store");
        client_builder = client_builder.cookie_store(true);
    }

    if config.ssl.ignore_ssl {
        warn!("SSL validation has been disabled, this is dangerous as all invalid certs would be accepted");
        client_builder = client_builder.danger_accept_invalid_certs(true);
    } else {
        if config.ssl.accept_invalid_hostnames {
            warn!("SSL hostname validation has been disabled, this is dangerous as all certs with non matching hostnames would be accepted");
            client_builder = client_builder.danger_accept_invalid_hostnames(true);
        }
    
        if config.ssl.certificate != "" {
            let cert = get_certificate(&config.ssl.certificate).await?;
            info!("Adding new trusted certificate {}", &config.ssl.certificate);
            client_builder = client_builder.add_root_certificate(cert);
        }

        if config.ssl.keystore != "" {
            let ks = get_identity(&config.ssl.keystore, &config.ssl.keystore_password).await?;
            info!("Adding new keystore {}", &config.ssl.keystore);
            client_builder = client_builder.identity(ks);
        }
    }

    Ok(client_builder.build()?)
}

fn get_header_map_from_request(request: &Request) 
-> Result<HeaderMap, Box<dyn Error + Send + Sync>> {
    let mut headers = HeaderMap::new();
    for header in &request.headers {
        headers.insert(HeaderName::from_str(header.0.as_str().unwrap())?, 
        HeaderValue::from_str(header.1.as_str().unwrap())?);
    }

    Ok(headers)
}

async fn add_multipart_form_data(builder: RequestBuilder, body: &Body) 
-> Result<RequestBuilder, Box<dyn Error + Send + Sync>> {
    let mut form = Form::new();

    for data in &body.formdata {
        let param_key = data.0.as_str().unwrap();
        let param_value = data.1.as_str().unwrap();

        if param_key.to_lowercase().starts_with("_file") {
            let contents = file::get_content(param_value).await?;
            let file_name = file::get_file_name(param_value)?;
            let part = Part::stream(contents).file_name(file_name)
                                .mime_str("application/octet-stream").unwrap();
            form = form.part("", part);
        } else {
            form = form.text(param_key.to_owned(), param_value.to_owned());
        }
    }

    Ok(builder.multipart(form))
}

fn add_url_encoded_data(builder: RequestBuilder, body: &Body) -> RequestBuilder {
    let mut params = HashMap::with_capacity(body.urlencoded.len());

    &body.urlencoded.iter().for_each(|(k,v)| {
        params.insert(k.as_str().unwrap().to_owned(), v.as_str().unwrap().to_owned());
    });

    builder.form(&params)
}

pub async fn execute(client: &Client, request: &Request) -> Result<(Response, u128), Box<dyn Error + Send + Sync>>  {
   
    //Check if method is valid, else return error
    let method_name = &request.method.to_uppercase();
    let method = Method::from_str(method_name)?;
    if !is_method_valid(&method_name) {
        return Err(format!("Invalid method {} found for request {}", method_name, request.name).into())
    }

    //Create HeaderMap
    let headers = match get_header_map_from_request(&request) {
        Ok(headers) => headers,
        Err(err) => return Err(err)
    };

    //Create builder
    let mut builder = client.request(method, request.url.to_owned()).headers(headers);

    //Add required body
    let body = &request.body;

    if body.raw != ""  {
        builder = builder.body(body.raw.to_owned());
    } else if body.formdata.len() != 0 {
        builder = add_multipart_form_data(builder, body).await?;
    } else if body.urlencoded.len() != 0 {
        builder = add_url_encoded_data(builder, body);
    } 

    //Initialising timestamps
    let start_time = time::Instant::now();
    let resp = builder.send().await?;
    let end_time = start_time.elapsed().as_millis();
   
    Ok((resp, end_time))
}