use crate::parser;

use std::collections::HashMap;
use std::error::Error;

use ajson;
use log::{debug, error};
use regex::Regex;
use reqwest::{blocking::Response, header::{HeaderMap, CONTENT_TYPE}};
use serde_json::{Map, Value};
use sxd_xpath::{evaluate_xpath};
use sxd_document::parser as xml_parser;

pub fn process(response: Response, request: &parser::Request, env_map: &mut HashMap<String, String>) -> Result<(), Box<dyn Error + 'static>> {
    let extractor = &request.extractor;
    let is_json_response = is_json_response(&response);
    let is_xml_response = !is_json_response && is_xml_response(&response);
    let body = get_response_as_string(response);

    if is_json_response { //Check if response is json
        process_json_path(&body, &extractor.gjson_path, env_map)?; 
    } else if is_xml_response { //Check if response is xml / html
        process_xpath(&body, &extractor.xpath, env_map)?; 
    } 

    process_regex(&body, &extractor.regex, env_map)
}

fn process_json_path(body: &str, jp_map: &Map<String, Value>, env_map: &mut HashMap<String, String>) -> Result<(), Box<dyn Error + 'static>> {
    debug!("Executing Gjson path extractor");
    for param_name in jp_map.keys() {
        let jsonpath = jp_map.get(param_name).unwrap()
                                .as_str()
                                .ok_or(format!("Gjson path must be a string for key {}", param_name))?;

        debug!("Fetching value for jsonpath: {}", jsonpath);
        let param_value = ajson::get(body, jsonpath).ok_or(format!("No value found for path {}", jsonpath))?;
         
        debug!("Value fetched against json path {} : {:?}", jsonpath, param_value);
        env_map.insert(param_name.to_string(), String::from(param_value.as_str()));
    }

    Ok(())
}

fn process_xpath(body: &str, xp_map: &Map<String, Value>, env_map: &mut HashMap<String, String>) -> Result<(), Box<dyn Error + 'static>>{
    debug!("Executing Xpath extractor");
    for param_name in xp_map.keys() {
        let xpath = xp_map.get(param_name).unwrap()
                                .as_str()
                                .ok_or(format!("Xpath must be a string for key {}", param_name))?;

        debug!("Fetching value for xpath: {}", xpath);
       
        let package = xml_parser::parse(body)?;
        let document = package.as_document();
        let param_value = evaluate_xpath(&document, xpath)?;

        debug!("Value fetched against xpath {} : {:?}", xpath, param_value);
        env_map.insert(param_name.to_string(), param_value.into_string());
    }
    Ok(())
}

fn process_regex(body: &str, regex_map: &Map<String, Value>, env_map: &mut HashMap<String, String>) -> Result<(), Box<dyn Error + 'static>> {
    debug!("Executing RegEx extractor");
    for param_name in regex_map.keys() {
        let regex = regex_map.get(param_name).unwrap()
                                .as_str()
                                .ok_or(format!("Regex must be a string for key {}", param_name))?;

        debug!("Fetching value for regex: {}", regex);

        let re = Regex::new(&format!(r"{}", regex))?;
        let param_value = re.find(body).ok_or(format!("No match found for regex {}", re.as_str()))?;

        debug!("Value fetched against regex {} : {:?}", regex, param_value);
        env_map.insert(param_name.to_string(), String::from(param_value.as_str()));
    }
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