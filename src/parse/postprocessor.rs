use jsonpath_lib as jsonpath;
use libxml::parser::Parser as xml_parser;
use libxml::xpath::Context;
use log::{debug, error, warn};
use regex::Regex;
use reqwest::{
    Response, 
    header::{HeaderMap, CONTENT_TYPE}
};
use serde_yaml::{Mapping, Value};

use std::{
    collections::HashMap,
    error::Error,
    fmt,
};

use crate::model;

#[derive(Debug)]
enum ProcessorType {
    JsonPath,
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
        let value = v.as_str().ok_or(format!("Value for extractor {} must be a string", k))?;
        debug!("Fetching value for {}: {}", self.to_string(), value);
        
        use ProcessorType::*;
        let param_value = match *self {
            JsonPath => execute_gjson_path(value, body)?,
            XmlPath =>  execute_xpath(value, body)?,
            RegEx => execute_regex(value, body)?
        };

        debug!("Value fetched against {} {} : {}", self.to_string(), value, param_value);
        Ok(param_value)
    }
}

fn execute_regex(pattern: &str, body: &str) -> Result<String, Box<dyn Error + 'static>> {
    let re = Regex::new(&format!(r"{}", pattern))?;
    let regex_match = re.find(body).ok_or(format!("No match found for regex {}", re.as_str()))?.as_str();
    Ok(String::from(regex_match))
}

fn execute_gjson_path(jsonpath: &str, body: &str) -> Result<String, Box<dyn Error + 'static>> {
    let json: serde_json::Value = serde_json::from_str(body).unwrap();
    let mut selector = jsonpath::selector(&json);

    let val_from_jsonpath = match selector(jsonpath) {
        Ok(val) => val,
        Err(err) => return Err(Box::new(err))
    };

    if val_from_jsonpath.is_empty() {
        return Err(format!("unable to retrieve data using jsonpath {}", jsonpath).into());
    }

    let val_as_str = val_from_jsonpath[0].as_str().unwrap();
    Ok(String::from(val_as_str))
}

fn execute_xpath(xpath: &str, body: &str) -> Result<String, Box<dyn Error + 'static>> {
    let parser: xml_parser = match body.contains("<html>.*</html>") {
        true => xml_parser::default_html(),
        _ => xml_parser::default()
    };
    
    let doc = match parser.parse_string(body) {
        Ok(doc) => doc,
        Err(err) => return Err(format!("Unable to parse body for xpath {}: {}", xpath, err).into())
    };

    let context = Context::new(&doc).unwrap();
    let nodes = context.evaluate(xpath).unwrap().get_nodes_as_vec();

    if nodes.len() == 0 {
        return Err(format!("No results found for xpath {}", xpath).into());
    } else if nodes.len() > 1 {
        warn!("Xpath {} matches multiple nodes. only first node would be considered", xpath)
    }

    Ok(nodes[0].get_content())
}

fn extract(processor_type: ProcessorType, body: &str, map: &Mapping, env_map: &mut HashMap<String, String>)
 -> Result<(), Box<dyn Error + 'static>> {
    for (k, v) in map {
        let k_as_str = k.as_str().unwrap();
        let param_value = processor_type.extractor(k_as_str,v,body)?;
        env_map.insert(k_as_str.to_string(), param_value);
    }

    Ok(())
}

pub async fn process(response: Response, request: &model::scenarios::Request, env_map: &mut HashMap<String, String>) -> Result<(), Box<dyn Error + 'static>> {
    let extractor = &request.extractor;
    let is_json_response = is_json_response(&response);
    let is_xml_response = !is_json_response && is_xml_response(&response);
    let body = get_response_as_string(response).await;

    if is_json_response { //Check if response is json
        extract(ProcessorType::JsonPath, &body, &extractor.gjson_path, env_map)?; 
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

async fn get_response_as_string(response: Response) -> String {
    match response.text().await {
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