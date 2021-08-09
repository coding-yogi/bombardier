use serde::{Serialize, Deserialize};
use serde_yaml::Mapping;

#[derive(Deserialize, Debug)]
pub struct Root {
    pub version: String,
    pub scenarios: Vec<Scenario>
}

#[derive(Deserialize, Debug)]
pub struct Environment {
    pub variables: Mapping
}

#[derive(Deserialize, Debug)]
pub struct Scenario {
    pub name: String,

    #[serde(rename = "threadCount")]
    pub thread_count: i32,

    #[serde(default)]
    pub requests: Vec<Request>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Request {
    pub name: String,
    pub url: String,
    pub method: String,

    #[serde(default)]
    pub headers: Mapping,

    #[serde(default)]
    pub body: Body,

    #[serde(default)]
    pub extractor: Extractor
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Body {
    #[serde(default)]
    pub raw: String,

    #[serde(default)]
    pub urlencoded: Mapping,

    #[serde(default)]
    pub formdata: Mapping,
}

/*#[derive(Deserialize, Debug)]
pub struct Env {
    #[serde(rename = "values")]
    pub key_values: Vec<KeyValue>
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct KeyValue {
    pub key: String,
    pub value: String
}*/

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Extractor {
    #[serde(default, rename = "jsonPath")]
    pub gjson_path: Mapping,
    
    #[serde(default)]
    pub xpath: Mapping,

    #[serde(default)]
    pub regex: Mapping
}