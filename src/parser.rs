use crate::file;
use std::process;
use std::collections::HashMap;

use log::{debug,error};
use regex::Regex;
use serde::{Serialize, Deserialize};
use serde_json::{Map, Value};

#[derive(Deserialize, Debug)]
pub struct Root {
    #[serde(rename = "item")]
    pub scenarios: Vec<Scenario>,
}

#[derive(Deserialize, Debug)]
pub struct Scenario {
    pub name: String,

    #[serde(default)]
    pub event: Vec<Event>,

    #[serde(rename = "request", default)]
    pub request_details: RequestDetails,

    #[serde(rename = "item", default)]
    pub requests: Vec<Request>,

    #[serde(default)]
    pub extractor: Extractor
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Request {
    pub name: String,

    #[serde(rename = "request")]
    pub request_details: RequestDetails,

    #[serde(default)]
    pub event: Vec<Event>,

    #[serde(default)]
    pub extractor: Extractor
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

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Event {
    pub listen: String,
    pub script: Script
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Script {
    pub exec: Vec<String>
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Extractor {
    #[serde(default)]
    pub json_path: Map<String, Value>,
    
    #[serde(default)]
    pub xpath: Map<String, Value>,

    #[serde(default)]
    pub regex: Map<String, Value>,
}

pub fn parse_requests(content: String, env_map: &HashMap<String, String>) -> Vec<Request> {
    let json = file::find_and_replace(content, &env_map);
    let root: Root = serde_json::from_str(&json).expect("Unable to parse Json");

    let mut requests = Vec::<Request>::new();
    /*bombardier_requests = root.scenarios.iter()
                            .filter(|s| s.request_details.method != "")
                            .map(|s| BombardierRequest::new(s))
                            .collect();*/

    for scenario in root.scenarios {
        if scenario.request_details.method != "" {
            let mut request = get_request_from_scenario(&scenario);
            request = get_extractor_json(request);
            requests.push(request);
        }
        
        for mut request in scenario.requests {
            request = get_extractor_json(request);
            requests.push(request);
        }
    } 

    requests
}

fn get_request_from_scenario(scenario: &Scenario) -> Request {
    Request {
        name: scenario.name.clone(),
        event: scenario.event.clone(),
        request_details: scenario.request_details.clone(),
        extractor: scenario.extractor.clone()
    }
}

pub fn get_env(env_file: &str) -> HashMap<String, String> {
    let config_content = file::get_content(env_file);
    let env_json: Env = serde_json::from_str(&config_content).expect("Unable to parse Json");
    let mut env_map: HashMap<String, String> = HashMap::new();
    for kv in env_json.key_values {
        env_map.insert(kv.key, kv.value);
    }

    env_map
}

fn get_extractor_json(mut request: Request) -> Request {
    let script = get_script(&request);

    let pattern = r#"var\s+bombardier\s*=\s*([{]{1}([,:{}\[\]0-9.\-+Eaeflnr-u \n\r\t]|".*?")+}{1})"#;
    let re = Regex::new(pattern).unwrap();
    let extractor_json = match re.captures(&script) {
        Some(cap) => cap[1].to_string(),
        None => String::from("")
    };

    if extractor_json != "" {
        debug!("Extractor json found for request {}: {}", request.name, extractor_json);
        let extractor: Extractor = match serde_json::from_str(&extractor_json) {
            Err(err) => {
                error!("Json provider as bombardier variable for request {} is not valid: {}", request.name, err);
                process::exit(-1)
            },
            Ok(e) => e
        };
    
        request.extractor = extractor;
    }
    
    request
}

fn get_script(request: &Request) -> String {
    match request.event.iter().find(|e|  e.listen == "test") {
        None => String::from(""),
        Some(s) => {
            s.script
            .exec
            .iter()
            .flat_map(|s| s.chars())
            .collect()
        }
    }
}