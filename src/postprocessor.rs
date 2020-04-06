use crate::parser;

use std::collections::HashMap;

use ajson;
use log::{debug, error};
use reqwest::{blocking::Response, header::{HeaderMap, CONTENT_TYPE}};
use serde_json::{Map, Value};

pub fn process(response: Response, request: &parser::Request, env_map: &mut HashMap<String, String>) -> Result<(), String> {
    let extractor = &request.extractor;
    let is_json_response = is_json_response(&response);
    let is_xml_response = is_xml_response(&response);
    let body = get_response_as_string(response);

    if is_json_response { //Check if response is json
        process_json_path(&body, &extractor.gjson_path, env_map)?; 
    } else if is_xml_response { //Check if response is xml / html
        process_xpath(&body, &extractor.xpath, env_map)?; 
    } 

    process_regex(&body, &extractor.regex, env_map)
}

fn process_json_path(body: &str, jp_map: &Map<String, Value>, env_map: &mut HashMap<String, String>) -> Result<(), String> {
    debug!("Executing Gjson path extractor");
    for k in jp_map.keys() {
        let param_name = k.to_string();

        let json_path = jp_map.get(k).unwrap(); 
        let json_path_as_str = json_path.as_str().ok_or(format!("Gjson path must be a string for key {}", k))?;

        debug!("Fetching value for jsonpath: {}", json_path_as_str);
        let param_value = ajson::get(body, json_path_as_str).ok_or(format!("No value found for path {}", json_path_as_str))?;
         
        debug!("Value fetched against json path {} : {:?}", json_path_as_str, param_value);
        env_map.insert(param_name, String::from(param_value.as_str()));
    }

    Ok(())
}

fn process_xpath(body: &str, xp_map: &Map<String, Value>, env_map: &mut HashMap<String, String>) -> Result<(), String> {
    Ok(())
}

fn process_regex(body: &str, regex_map: &Map<String, Value>, env_map: &mut HashMap<String, String>) -> Result<(), String> {
    Ok(())
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