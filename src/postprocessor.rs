use crate::parser;

use std::collections::HashMap;
use std::error;

use ajson;
use log::{debug, error};
use reqwest::{blocking::Response, header::{HeaderMap, CONTENT_TYPE}};
use serde_json::{Map, Value};

pub fn process(response: Response, request: &parser::Request, env_map: &mut HashMap<String, String>) {
    let extractor = &request.extractor;
    if extractor.gjson_path.len() > 0 {
        match process_json_path(response, &extractor.gjson_path, env_map) {
            Ok(_) => (),
            Err(err) => error!("Json path processing failed: Error: {}", err)
        }
    } else if extractor.xpath.len() > 0 {
        process_xpath();
    }

    if extractor.regex.len() > 0 {
        process_regex();
    }
}

fn process_json_path(response: Response, jp_map: &Map<String, Value>, env_map: &mut HashMap<String, String>) -> Result<(), Box<dyn error::Error + 'static>> {
    debug!("Executing Gjson path extractor");
    if is_json_response(&response) {
        let body = get_response_as_string(response);
        for k in jp_map.keys() {
            let param_name = k.to_string();

            let json_path = jp_map.get(k).unwrap(); 
            let json_path_as_str = match json_path.as_str() {
                Some(s) => s,
                None => return Err(format!("Gjson path must be a string for key {}", k).into())
            };

            debug!("Fetching value for jsonpath: {}", json_path_as_str);
            let param_value = match ajson::get(&body, json_path_as_str) {
                Some(v) => v,
                None => {
                    return Err(format!("No value found for path {}", json_path_as_str).into())
                }
            };

            debug!("Value fetched against json path {} : {:?}", json_path_as_str, param_value);
            env_map.insert(param_name, String::from(param_value.as_str()));
        }

        Ok(())
    } else {
        Err("Gjson path extractor defined but response is not a JSON".into())
    }
}

fn process_xpath() {

}

fn process_regex() {

}

fn is_json_response(response: &Response) -> bool {
    let content_type = get_response_content_type(&response.headers());
    content_type.contains("application/json")
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