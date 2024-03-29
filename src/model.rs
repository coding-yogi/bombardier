use serde::{Serialize, Deserialize, Deserializer, de::Error};
use rustc_hash::FxHashMap as HashMap;

//Config is the model for execution configuration
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_to_one")]
    #[serde(rename = "threadCount")]
    pub thread_count: u32,

    #[serde(default)]
    pub iterations: u64,

    #[serde(default)]
    #[serde(rename = "executionTime")]
    pub execution_time: u64,

    #[serde(default = "default_to_one")]
    #[serde(rename = "thinkTime")]
    pub think_time: u32,

    #[serde(deserialize_with = "check_non_zero")]
    #[serde(rename = "rampUpTime")]
    #[serde(default = "default_to_one")]
    pub rampup_time: u32,
    
    #[serde(default)]
    #[serde(rename = "handleCookies")]
    pub handle_cookies: bool,

    #[serde(default)]
    #[serde(rename = "continueOnError")]
    pub continue_on_error: bool,

    #[serde(default)]
    pub database: Database,

    #[serde(default)]
    pub ssl: Ssl,

    #[serde(skip_deserializing)]
    #[serde(skip_serializing)]
    pub distributed: bool,

    #[serde(skip_deserializing)]
    #[serde(skip_serializing)]
    pub data_file: String,

    #[serde(skip_deserializing)]
    #[serde(skip_serializing)]
    pub report_file: String
}

fn check_non_zero <'de, D>(deserializer: D) -> Result<u32, D::Error> 
where D: Deserializer<'de> {    
    let val = u32::deserialize(deserializer)?;
    if val == 0 {
        return Err(Error::custom("Value cannot be zero"))
    }

    Ok(val)
}

fn default_to_one() -> u32 {
    1
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Database {
    #[serde(rename = "type")]
    #[serde(default)]
    pub db_type: String,

    #[serde(default)]
    pub url: String,

    #[serde(default)]
    pub user: String,

    #[serde(default)]
    pub password: String,

    #[serde(default)]
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Ssl {
    #[serde(default)]
    #[serde(rename = "ignoreSSL")]
    pub ignore_ssl: bool,

    #[serde(default)]
    #[serde(rename = "acceptInvalidHostnames")]
    pub accept_invalid_hostnames: bool,

    #[serde(default, deserialize_with = "check_der_or_pem")]
    pub certificate: String,

    #[serde(default, deserialize_with = "check_p12_or_pfx")]
    pub keystore: String,

    #[serde(default)]
    #[serde(rename = "keystorePassword")]
    pub keystore_password: String,
}

fn check_der_or_pem <'de, D>(deserializer: D) -> Result<String, D::Error> 
where D: Deserializer<'de> {   
    let val = String::deserialize(deserializer)?;
    if !(val.is_empty() || val.ends_with(".der") || val.ends_with(".pem")){
        return Err(Error::custom("File should be a .pem or .der file"))
    }

    Ok(val)
}

fn check_p12_or_pfx <'de, D>(deserializer: D) -> Result<String, D::Error> 
where D: Deserializer<'de> {   
    let val = String::deserialize(deserializer)?;
    if !(val.is_empty() || val.ends_with(".p12") || val.ends_with(".pfx")){
        return Err(Error::custom("File should be a .p12 or .pfx file"))
    }

    Ok(val)
}

#[derive(Deserialize, Debug)]
pub struct Root {
    pub version: String,
    pub scenarios: Vec<Scenario>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Environment {
    pub variables: HashMap<String, String>
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

    pub url: String,

    pub method: String,

    #[serde(default)]
    pub headers: HashMap<String, String>,

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
    pub urlencoded: HashMap<String, String>,

    #[serde(default)]
    pub formdata: Vec<FormDataField>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct FormDataField {
    #[serde(default)]
    pub name: String,

    #[serde(default)]
    #[serde(rename = "type")]
    pub field_type: FormDataFieldType,

    #[serde(default)]
    pub value: String,

    #[serde(default)]
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum FormDataFieldType {
    Text,
    File
}

impl Default for FormDataFieldType {
    fn default() -> Self {
        FormDataFieldType::Text
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Extractor {
    #[serde(default)]
    pub from: ExtractFrom,

    #[serde(rename = "type")]
    #[serde(default)]
    pub extractor_type: ExtractorType,

    #[serde(default)]
    pub extract: HashMap<String, String>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ExtractFrom {
    Body,
    Headers
}

impl Default for ExtractFrom {
    fn default() -> Self {
        ExtractFrom::Body
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ExtractorType {
    GjsonPath,
    Xpath,
    RegEx,
    None
}

impl Default for ExtractorType {
    fn default() -> Self {
        ExtractorType::None
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Env {
    pub key: String,
    pub value: String
}