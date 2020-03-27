use std::collections::HashMap;
use serde::{Serialize, Deserialize};

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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Request {
    pub name: String,

    #[serde(rename = "request")]
    pub request_details: RequestDetails
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct RequestDetails {
    pub method: String,
    pub url: Url,

    #[serde(default)]
    pub body: Body,

    #[serde(default)]
    pub auth: Auth,

    #[serde(rename = "header")]
    pub headers: Vec<KeyValue>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Url {
    pub raw: String
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Auth {
    #[serde(rename = "type")]
    pub auth_type: String,

    #[serde(default)]
    pub basic: Vec<KeyValue>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FormData {
    pub key: String,

    #[serde(default)]
    pub value: String,

    #[serde(rename = "type")]
    pub param_type: String,

    #[serde(default)]
    pub src: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct KeyValue {
    pub key: String,
    pub value: String
}

pub fn parse_requests(json: &str) -> Vec<Request> {
    let root: Root = serde_json::from_str(&json).expect("Unable to parse Json");

    let mut requests = Vec::<Request>::new();
    /*bombardier_requests = root.scenarios.iter()
                            .filter(|s| s.request_details.method != "")
                            .map(|s| BombardierRequest::new(s))
                            .collect();*/

    for scenario in root.scenarios {
        if scenario.request_details.method != "" {
            requests.push(get_request_from_scenario(&scenario));
        }
        
        for request in scenario.requests {
            requests.push(request);
        }
    } 

    requests
}

fn get_request_from_scenario(scenario: &Scenario) -> Request {
    Request {
        name: scenario.name.clone(),
        request_details: scenario.request_details.clone()
    }
}

pub fn get_env(json: &str) -> HashMap<String, String> {
    let env_json: Env = serde_json::from_str(&json).unwrap();
    let mut env_map: HashMap<String, String> = HashMap::new();
    for kv in env_json.key_values {
        env_map.insert(kv.key, kv.value);
    }

    env_map
}