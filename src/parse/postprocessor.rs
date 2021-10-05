use gjson;
use libxml::parser::Parser as xml_parser;
use libxml::xpath::Context;
use log::{debug, error, warn};
use regex::Regex;
use reqwest::{
    Response, 
    header::{HeaderMap, CONTENT_TYPE}
};
use rustc_hash::FxHashMap as HashMap;
use serde_yaml::Mapping;

use std::error::Error;

use crate::model;

trait Extractor {
    fn extract(&self, pattern: &str, body: &str) -> Result<String, Box<dyn Error + 'static>>;
    fn name(&self) -> &'static str;
}

struct JsonExtractor;

impl Extractor for JsonExtractor {
    fn extract(&self, jsonpath: &str, body: &str) -> Result<String, Box<dyn Error + 'static>> {
        let val_from_jsonpath = gjson::get(body, jsonpath);
        Ok(val_from_jsonpath.to_string())
    }

    fn name(&self) -> &'static str{
        "gjsonpath"
    }
}

struct XpathExtractor;

impl Extractor for XpathExtractor {
    fn extract(&self, xpath: &str, body: &str) -> Result<String, Box<dyn Error + 'static>> {
        let parser: xml_parser = match body.contains("<html>.*</html>") {
            true => xml_parser::default_html(),
            _ => xml_parser::default()
        };
        
        let doc = match parser.parse_string(body) {
            Ok(doc) => doc,
            Err(err) => return Err(format!("Unable to parse body for xpath {} from body {}: {}", xpath, body, err).into())
        };
    
        let context = Context::new(&doc).unwrap();
        let nodes = context.evaluate(xpath).unwrap().get_nodes_as_vec();
    
        if nodes.is_empty() {
            return Err(format!("No results found for xpath {}", xpath).into());
        } else if nodes.len() > 1 {
            warn!("Xpath {} matches multiple nodes. only first node would be considered", xpath)
        }
    
        Ok(nodes[0].get_content())
    }

    fn name(&self) -> &'static str{
        "xpath"
    }
}

struct RegExExtractor;

impl Extractor for RegExExtractor {
    fn extract(&self, pattern: &str, body: &str) -> Result<String, Box<dyn Error + 'static>> {
        let re = Regex::new(pattern)?;
        let regex_match = re.find(body).ok_or(format!("No match found for regex {}", re.as_str()))?.as_str();
        Ok(String::from(regex_match))
    }

    fn name(&self) -> &'static str{
        "regex"
    }
}

fn extract<T: Extractor>(extractor: T, body: &str, map: &Mapping, env_map: &mut HashMap<String, String>)
 -> Result<(), Box<dyn Error + 'static>> {
    for (k, v) in map {
        let keyname = k.as_str().ok_or("Key for extractor must be a string")?;
        let pattern = v.as_str().ok_or(format!("Value for extractor {} must be a string", keyname))?;
        debug!("Fetching value for {}: {}", extractor.name(), pattern);
        let extracted_value = extractor.extract(pattern,body)?;
        debug!("Value fetched against {} {} : {}", extractor.name(), pattern, extracted_value);
        env_map.insert(keyname.to_string(), extracted_value);
    }

    Ok(())
}

pub async fn process(response: Response, extractors: &[model::Extractor], env_map: &mut HashMap<String, String>) -> Result<(), Box<dyn Error + 'static>> {
    let is_json_response = is_json_response(&response);
    let is_xml_response = !is_json_response && is_xml_response(&response);
    let body = get_response_as_string(response).await; 
    
    for extractor in extractors { 
        match extractor.extractor_type.as_str() {
            "gjsonpath"  => {
                if !is_json_response {
                    error!("Response is not in json format, json extractor will not be executed");
                    continue;
                }

                extract(JsonExtractor, &body, &extractor.extract, env_map)?; 
            },
            "xpath" => {
                if !is_xml_response {
                    error!("Response is not in xml/html format, xml extractor will not be executed");
                    continue;
                }

                extract(XpathExtractor, &body, &extractor.extract, env_map)?; 
            },
            "regex" => {
                extract(RegExExtractor, &body, &extractor.extract, env_map)?; 
            },
            _ => {
                error!("Invalid extractor type found: {}", extractor.extractor_type.as_str())
            }
        }  
    }

    Ok(())
}

fn is_json_response(response: &Response) -> bool {
    let content_type = get_response_content_type(response.headers());
    content_type.contains("json")
}

fn is_xml_response(response: &Response) -> bool {
    let content_type = get_response_content_type(response.headers());
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

fn get_response_content_type(headers: &HeaderMap) -> &str {
    match headers.get(CONTENT_TYPE) {
        Some(v) => {
            match v.to_str() {
                Ok(s) => s,
                Err(err) => {
                    error!("Content-Type header not found: Error {}", err);
                    ""
                }
            }
        },
        None => ""
    }
}