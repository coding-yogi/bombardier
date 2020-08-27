use crate::cmd;
use crate::file;
use crate::parser;

use log::{debug, info, warn};
use reqwest::{blocking::{Client, Response}, Certificate, Identity, Method};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::str::FromStr;
use std::time;

pub fn get_default_sync_client() -> Client {
    Client::builder()
        .user_agent("bombardier")
        .build()
        .expect("Unable to create default sync client")
} 

fn get_certificate(path: &str)  -> Result<Certificate, Box<dyn std::error::Error + Send + Sync>> {
    let cert = file::read_file(path)?;
    if path.to_lowercase().ends_with(cmd::DER_EXT) {
        return Ok(Certificate::from_der(&cert)?)
    } else if path.to_lowercase().ends_with(cmd::PEM_EXT) {
        return Ok(Certificate::from_pem(&cert)?)
    }

    Err("Certificate should be in .pem or .der format".into())
}

fn get_identity(path: &str, password: &str) -> Result<Identity, Box<dyn std::error::Error + Send + Sync>> {
    let ks = file::read_file(path)?;
    if path.to_lowercase().ends_with(cmd::P12_EXT) || path.to_lowercase().ends_with(cmd::PFX_EXT) {
        return Ok(Identity::from_pkcs12_der(&ks, password)?)
    }

    Err("Keystore should be in .p12 or .pfx format".into())
}

pub fn get_sync_client(config: &cmd::ExecConfig)  -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
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
            let cert = get_certificate(&config.ssl.certificate)?;
            info!("Adding new trusted certificate {}", &config.ssl.certificate);
            client_builder = client_builder.add_root_certificate(cert);
        }
    }

    if config.ssl.keystore != "" {
        let ks = get_identity(&config.ssl.keystore, &config.ssl.keystore_password)?;
        info!("Adding new keystore {}", &config.ssl.keystore);
        client_builder = client_builder.identity(ks);
    }
    

    Ok(client_builder.build()?)
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
        let username = auth.basic.iter().find(|kv| kv.key == "username").unwrap().value.as_str();
        let password = auth.basic.iter().find(|kv| kv.key == "password").unwrap().value.as_str();
        if let Some(username) = username {
            builder = builder.basic_auth(username, password);
        }
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