use crate::parser;

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use ajson;
use log::{debug, error};
use regex::Regex;
use reqwest::{blocking::Response, header::{HeaderMap, CONTENT_TYPE}};
use serde_json::{Map, Value};
use sxd_xpath::{evaluate_xpath};
use sxd_document::parser as xml_parser;

#[derive(Debug)]
enum ProcessorType {
    GJsonPath,
    XmlPath,
    RegEx
}

impl fmt::Display for ProcessorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl ProcessorType {
    fn extractor(&self, k: &str, v: &Value, body: &str) -> Result<String, Box<dyn Error + 'static>> {
        let param_value: String;
        let value = v.as_str().ok_or(format!("Value for extractor {} must be a string", k))?;
        debug!("Fetching value for {}: {}", self.to_string(), value);
        
        use ProcessorType::*;
        match *self {
            GJsonPath => {
                let val_from_jsonpath = ajson::get(body, value).ok_or(format!("No value found for path {}", value))?;
                let val_as_str = val_from_jsonpath.as_str();
                param_value = String::from(val_as_str);
            },
            XmlPath => {
                let package = xml_parser::parse(body)?;
                let document = package.as_document();
                param_value = evaluate_xpath(&document, value)?.into_string();
            },
            RegEx => {
                let re = Regex::new(&format!(r"{}", value))?;
                let regex_match = re.find(body).ok_or(format!("No match found for regex {}", re.as_str()))?.as_str();
                param_value = String::from(regex_match);
            }
        }

        debug!("Value fetched against {} {} : {}", self.to_string(), value, param_value);
        Ok(param_value)
    }
}

fn extract(processor_type: ProcessorType, body: &str, map: &Map<String, Value>, env_map: &mut HashMap<String, String>)
 -> Result<(), Box<dyn Error + 'static>> {
    for (k, v) in map {
        let param_value = processor_type.extractor(k,v,body)?;
        env_map.insert(k.to_string(), param_value);
    }

    Ok(())
}

pub fn process(response: Response, request: &parser::Request, env_map: &mut HashMap<String, String>) -> Result<(), Box<dyn Error + 'static>> {
    let extractor = &request.extractor;
    let is_json_response = is_json_response(&response);
    let is_xml_response = !is_json_response && is_xml_response(&response);
    let body = get_response_as_string(response);

    if is_json_response { //Check if response is json
        extract(ProcessorType::GJsonPath, &body, &extractor.gjson_path, env_map)?; 
    } else if is_xml_response { //Check if response is xml / html
        extract(ProcessorType::XmlPath, &body, &extractor.xpath, env_map)?; 
    } 

    extract(ProcessorType::RegEx, &body, &extractor.regex, env_map)
}

fn is_json_response(response: &Response) -> bool {
    let content_type = get_response_content_type(&response.headers());
    content_type.contains("json")
}

fn is_xml_response(response: &Response) -> bool {
    let content_type = get_response_content_type(&response.headers());
    content_type.contains("xml") || content_type.contains("html")
}

fn get_response_as_string(response: Response) -> String {
    match response.text() {
        Ok(s) => s,
        Err(err) => {
            error!("Error while getting response as String: {}", err);
            String::from("")
        }
    }
}

fn get_response_content_type(headers: &HeaderMap) -> String {
    match headers.get(CONTENT_TYPE) {
        Some(v) => {
            match v.to_str() {
                Ok(s) => s.to_string(),
                Err(err) => {
                    error!("Content-Type header not found: Error {}", err);
                    String::from("")
                }
            }
        },
        None => String::from("")
    }
}