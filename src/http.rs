use crate::cmd;
use crate::parser;

use std::time;
use std::str::FromStr;
use std::collections::HashMap;

use log::{debug};
use reqwest::{blocking::{Client, Response}, Method};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

pub fn get_default_sync_client() -> Client {
    Client::builder()
        .user_agent("bombardier")
        .build()
        .expect("Unable to create default sync client")
} 

pub fn get_sync_client(config: &cmd::ExecConfig)  -> Client {
    let mut client_builder = Client::builder().user_agent("bombardier");

    if config.handle_cookies {
        client_builder = client_builder.cookie_store(true);
    }

    let client = client_builder.build()
        .expect("Unable to create client");

    client
}

pub fn execute(client: &Client, request: parser::Request) -> Result<(Response, u128), Box<dyn std::error::Error + Send + Sync>>  {
    let details = &request.request_details;
    let method = Method::from_bytes(details.method.as_bytes())?;
    let uri = &details.url.raw;
    let mut headers = HeaderMap::new();
    for header in &details.headers {
        headers.insert(HeaderName::from_str(header.key.as_ref())?, 
        HeaderValue::from_str(header.value.as_str())?);
    }

    let mut builder = client.request(method, uri).headers(headers);

    let auth = &details.auth;
    if auth.auth_type == "basic" {
        let username = auth.basic.iter().find(|kv| kv.key == "username").unwrap().value.clone();
        let password = auth.basic.iter().find(|kv| kv.key == "password").unwrap().value.clone();
        builder = builder.basic_auth(username, Some(password));
    }

    match details.body.mode.as_ref() {
        "formdata" => {
            debug!("multipart form data found");
            let mut form = reqwest::blocking::multipart::Form::new();
            for data in &details.body.formdata {
                form = match data.param_type.as_ref() {
                    "text" => form.text(data.key.clone(), data.value.clone()),
                    "file" => form.file("file", &data.src)?,
                    _ => Err("form data should have either text or file param")?
                }
            }

            builder = builder.multipart(form)
        },
        "urlencoded" => {
            debug!("url encoded data found");
            let mut params = HashMap::new();
            for param in &details.body.urlencoded {
                params.insert(&param.key, &param.value);
            }

            builder = builder.form(&params)
        },
        "raw" => builder = builder.body(details.body.raw.clone()),
        _ => ()
    }

    let start_time = time::Instant::now();
    let resp = builder.send()?;
    let end_time = start_time.elapsed().as_millis();
   
    Ok((resp, end_time))
}