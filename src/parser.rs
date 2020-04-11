use crate::file;
use std::process;
use std::collections::HashMap;

use log::{error};
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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
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
    pub gjson_path: Map<String, Value>,
    
    #[serde(default)]
    pub xpath: Map<String, Value>,

    #[serde(default)]
    pub regex: Map<String, Value>,
}

pub fn parse_requests(content: String, env_map: &HashMap<String, String>) -> Result<Vec<Request>, std::io::Error> {
    let json = file::find_and_replace(content, &env_map);
    let root: Root = serde_json::from_str(&json)?;

    let mut requests = Vec::<Request>::new();
  
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

    Ok(requests)
}

fn get_request_from_scenario(scenario: &Scenario) -> Request {
    Request {
        name: scenario.name.clone(),
        event: scenario.event.clone(),
        request_details: scenario.request_details.clone(),
        extractor: scenario.extractor.clone()
    }
}

pub fn get_env_map(env_file: &str) -> Result<HashMap<String, String>, std::io::Error> {
    let config_content = file::get_content(env_file)?;
    let env_json: Env = serde_json::from_str(&config_content)?;
    let mut env_map: HashMap<String, String> = HashMap::new();
    for kv in env_json.key_values {
        env_map.insert(kv.key, kv.value);
    }

    Ok(env_map)
}

fn get_extractor_json(mut request: Request) -> Request {
    let script = get_test_script(&request);

    let pattern = r#"var\s+bombardier\s*=\s*([{]{1}([,:{}\[\]0-9.\-+Eaeflnr-u \n\r\t]|".*?")+}{1})"#;
    let re = Regex::new(pattern).unwrap();
    let extractor_json = match re.captures(&script) {
        Some(cap) => cap[1].to_string(),
        None => String::from("")
    };

    if extractor_json != "" {
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

fn get_test_script(request: &Request) -> String {
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

pub fn get_vec_data_map(data_file: &str) -> Result<Vec<HashMap<String, String>>, csv::Error> {
    let mut vec_data_map: Vec<HashMap<String, String>> = Vec::new();

    if data_file != "" {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .trim(csv::Trim::All)
            .from_path(data_file)?;

        let mut records_iterator = reader.records();

        let headers: Vec<String> = records_iterator
            .next()
            .unwrap()?
            .iter()
            .map(|s| s.to_string())
            .collect();


        vec_data_map = records_iterator.map(|record| {
            headers.iter()
                .zip(record.unwrap().iter())
                .map(|(k,v)| (k.clone(), v.to_string()))
                .collect()
        }).collect();
    }
    
    Ok(vec_data_map)
}