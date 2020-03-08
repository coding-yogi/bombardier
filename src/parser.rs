use serde::{Deserialize};
use serde_json::Result;
use std::collections::HashMap;

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

#[derive(Deserialize, Debug, Default)]
pub struct Request {
    pub name: String,

    #[serde(rename = "request")]
    pub request_details: RequestDetails
}

#[derive(Deserialize, Debug, Default)]
pub struct RequestDetails {
    pub method: String,
    pub url: Url,

}

#[derive(Deserialize, Debug, Default)]
pub struct Url {
    pub raw: String
}

#[derive(Deserialize, Debug)]
pub struct RequestBody {
    pub mode: String,
    pub raw: String
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

//only for testing
pub fn get_scenarios(json: &str) -> Result<Root> {
    serde_json::from_str(&json)
}

pub fn get_env(json: &str) -> HashMap<String, String> {
    let env_json: Env = serde_json::from_str(&json).unwrap();
    let mut env_map: HashMap<String, String> = HashMap::new();
    for kv in env_json.key_values {
        env_map.insert(kv.key, kv.value);
    }

    env_map
}