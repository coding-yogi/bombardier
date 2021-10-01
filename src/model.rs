use serde::{Serialize, Deserialize};
use serde_yaml::Mapping;

#[derive(Deserialize, Debug)]
pub struct Root {
    pub version: String,
    pub scenarios: Vec<Scenario>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Environment {
    pub variables: Mapping
}

#[derive(Deserialize, Debug)]
pub struct Scenario {
    pub name: String,

    #[serde(default)]
    pub requests: Vec<Request>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request {
    #[serde(skip_deserializing)]
    #[serde(skip_serializing)]
    pub id: uuid::Uuid,

    pub name: String,

    pub url: url::Url,

    pub method: String,

    #[serde(default)]
    pub headers: Mapping,

    #[serde(default)]
    pub body: Body,

    #[serde(default)]
    pub extractors: Vec<Extractor>,

    #[serde(default)]
    pub requires_preprocessing: bool
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

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Extractor {
    #[serde(rename = "type")]
    pub extractor_type: String,

    pub extract: Mapping,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Env {
    pub key: String,
    pub value: String
}