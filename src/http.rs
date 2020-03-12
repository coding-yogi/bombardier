
use crate::parser;

use reqwest::{blocking::{Client, Body}, Method};
use reqwest::header::{HeaderMap, HeaderName};
use std::str::FromStr;

pub fn get_sync_client()  -> Client {
    let client = Client::builder()
        .user_agent("bombardier")
        .build()
        .expect("Unable to create client");

    client
}

pub fn execute(client: &Client, bmr: &parser::BombardierRequest) -> Result<(), Box<dyn std::error::Error + Send + Sync>>  {
    let method = Method::from_bytes(bmr.method.as_bytes()).unwrap();
    let uri = &bmr.url;
    let mut headers = HeaderMap::new();
    let body = Body::from(bmr.body.clone());
    for k in bmr.headers.keys() {
        headers.insert(HeaderName::from_str(k.as_ref()).unwrap(), bmr.headers.get(k).unwrap().parse().unwrap());
    }
    let builder = client.request(method, uri).headers(headers).body(body);
    let resp = builder.send().unwrap();
   
    Ok(())
}