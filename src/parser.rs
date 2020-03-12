use crate::file;

use std::collections::HashMap;
use serde::{Deserialize};
use base64::{encode_config, STANDARD};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

pub trait HasRequestDetails {
    fn get_request_details(&self) -> &RequestDetails;
    fn get_name(&self) -> String;
}

#[derive(Deserialize, Debug)]
pub struct Root {
    #[serde(rename = "item")]
    pub scenarios: Vec<Scenario>,
}

#[derive(Deserialize, Debug)]
pub struct Scenario {
    pub name: String,

    #[serde(rename = "request", default)]
    pub request_details: RequestDetails,

    #[serde(rename = "item", default)]
    pub requests: Vec<Request>
}

impl HasRequestDetails for Scenario {
    fn get_request_details(&self) -> &RequestDetails {
        &self.request_details
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct Request {
    pub name: String,

    #[serde(rename = "request")]
    pub request_details: RequestDetails
}

impl HasRequestDetails for Request {
    fn get_request_details(&self) -> &RequestDetails {
        &self.request_details
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct RequestDetails {
    pub method: String,
    pub url: Url,
    pub body: Body,

    #[serde(default)]
    pub auth: Auth,

    #[serde(rename = "header")]
    pub headers: Vec<KeyValue>,
}

impl RequestDetails {
    fn headers_as_map(&self) -> HashMap<String, String> {
        let mut hm = HashMap::new();
        for header in &self.headers {
            hm.insert(header.key.clone(), header.value.clone());
        }

        hm
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct Url {
    pub raw: String
}

#[derive(Deserialize, Debug, Default)]
pub struct Auth {
    #[serde(rename = "type")]
    pub auth_type: String,

    #[serde(default)]
    pub basic: Vec<KeyValue>,
}

#[derive(Deserialize, Debug, Default)]
pub struct Body {
    #[serde(default)]
    pub mode: String,

    #[serde(default)]
    pub raw: String,

    #[serde(default)]
    pub urlencoded: Vec<KeyValue>,

    #[serde(default)]
    pub formdata: Vec<FormData>,
}

#[derive(Deserialize, Debug)]
pub struct Env {
    #[serde(rename = "values")]
    pub key_values: Vec<KeyValue>
}

#[derive(Deserialize, Debug)]
pub struct FormData {
    pub key: String,

    #[serde(default)]
    pub value: String,

    #[serde(rename = "type")]
    pub param_type: String,

    #[serde(default)]
    pub src: String,
}

#[derive(Deserialize, Debug)]
pub struct KeyValue {
    pub key: String,
    pub value: String
}

#[derive(Debug)]
pub struct BombardierRequest {
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl BombardierRequest {
    fn new<T: HasRequestDetails>(item: &T) -> BombardierRequest {
        let item_details = &item.get_request_details();
        let mut headers = item_details.headers_as_map();
        inject_basic_auth(item_details, &mut headers);
        
        BombardierRequest {
            name: item.get_name(),
            method: item_details.method.clone(),
            url: item_details.url.raw.clone(),
            headers: headers,
            body: stringify_body(item_details)
        }
    }
}

pub fn parse_requests(json: &str) -> Vec<BombardierRequest> {
    let root: Root = serde_json::from_str(&json).expect("Unable to parse Json");

    let mut bombardier_requests = Vec::<BombardierRequest>::new();
    /*bombardier_requests = root.scenarios.iter()
                            .filter(|s| s.request_details.method != "")
                            .map(|s| BombardierRequest::new(s))
                            .collect();*/

    for scenario in root.scenarios {
        if scenario.request_details.method != "" {
            bombardier_requests.push(BombardierRequest::new(&scenario));
        }
        
        for request in scenario.requests {
            bombardier_requests.push(BombardierRequest::new(&request));
        }
    } 

    bombardier_requests
}

fn inject_basic_auth(item_details: &RequestDetails, headers: &mut HashMap<String, String>) {
    let auth = &item_details.auth;
    if auth.auth_type == "basic" {
        
        let username = auth.basic
            .iter()
            .find(|kv| kv.key == "username")
            .unwrap()
            .value.clone();
        
        let password = auth.basic
            .iter()
            .find(|kv| kv.key == "password")
            .unwrap()
            .value.clone();

        let basic_auth =  encode_config(format!("{}:{}", &username, &password), STANDARD);
        headers.insert(String::from("authorization"), format!("Basic {}",basic_auth));
    }
}

fn stringify_body(item_details: &RequestDetails) -> String {
    match item_details.body.mode.as_ref() {
        "raw" => item_details.body.raw.to_owned(),
        "urlencoded" => stringify_url_encoded_body(&item_details.body.urlencoded),
        "formdata" => stringify_form_data_body(&item_details),
        _ => String::from("")
    }
}

fn stringify_url_encoded_body(urlencoded: &Vec<KeyValue>) -> String {
    let mut body = Vec::new();
    for param in urlencoded {
        body.push(format!("{}={}", param.key, param.value));
    }
        
    body.join("&")
}

fn stringify_form_data_body(request_details: &RequestDetails) -> String {
    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(15)
        .collect();

    let boundary = format!("----WebKitFormBoundary{}", rand_string);
    let mut body = Vec::new();
    for param in &request_details.body.formdata {
        match param.param_type.as_ref() {
            "text" => {
                body.push(format!("--{}\r\nContent-Disposition: form-data; name=\"{}\"\r\n\r\n{}\r\n",
                    boundary, param.key, param.value));
            },
            "file" => {
                body.push(format!("--{}\r\nContent-Disposition: form-data; name=\"\"; filename=\"{}\"\r\nContent-Type: {}\r\n\r\ntestdata\r\n",
                    boundary, file::get_file_name(&param.src), param.value));
            },
            _ => ()
        }                     
    }

    format!("{}--{}--",body.join(""), boundary)
}

pub fn get_env(json: &str) -> HashMap<String, String> {
    let env_json: Env = serde_json::from_str(&json).unwrap();
    let mut env_map: HashMap<String, String> = HashMap::new();
    for kv in env_json.key_values {
        env_map.insert(kv.key, kv.value);
    }

    env_map
}