use std::collections::HashMap;

use serde::{Deserialize};

pub trait HasRequestDetails {
    fn get_request_details(&mut self) -> &mut RequestDetails;
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
    fn get_request_details(&mut self) -> &mut RequestDetails {
        &mut self.request_details
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct Request {
    pub name: String,

    #[serde(rename = "request")]
    pub request_details: RequestDetails
}

impl HasRequestDetails for Request {
    fn get_request_details(&mut self) -> &mut RequestDetails {
        &mut self.request_details
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
    pub urlencoded: Vec<KeyValue>
}

#[derive(Deserialize, Debug)]
pub struct Env {
    #[serde(rename = "values")]
    pub key_values: Vec<KeyValue>
}

#[derive(Deserialize, Debug)]
pub struct KeyValue {
    pub key: String,
    pub value: String
}

pub fn parse_scenarios(json: &str) -> Vec<Scenario> {
    let root: Root = serde_json::from_str(&json).expect("Unable to parse Json");
    root.scenarios
}

pub fn get_env(json: &str) -> HashMap<String, String> {
    let env_json: Env = serde_json::from_str(&json).unwrap();
    let mut env_map: HashMap<String, String> = HashMap::new();
    for kv in env_json.key_values {
        env_map.insert(kv.key, kv.value);
    }

    env_map
}