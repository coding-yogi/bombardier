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

use std::error::Error;

use crate::model::{self, ExtractorType};

trait Extractor {
    fn extract(&self, pattern: &str, body: &str) -> Result<String, Box<dyn Error + 'static>>;
}

struct JsonExtractor;

impl Extractor for JsonExtractor {
    fn extract(&self, jsonpath: &str, body: &str) -> Result<String, Box<dyn Error + 'static>> {
        let val_from_jsonpath = gjson::get(body, jsonpath);
        debug!("Value fetched using jsonpath for {} : {}", jsonpath, val_from_jsonpath);
        Ok(val_from_jsonpath.to_string())
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
    
        let value = nodes[0].get_content();
        debug!("Value fetched using xpath for {} : {}", xpath, value);
        Ok(value)
    }
}

struct RegExExtractor;

impl Extractor for RegExExtractor {
    fn extract(&self, pattern: &str, body: &str) -> Result<String, Box<dyn Error + 'static>> {
        let re = Regex::new(pattern)?;
        let captures = re.captures(body).ok_or(format!("No match found for regex {}", pattern))?;

        let mut capture_group: usize = 0;

        if captures.len() > 1 {
            debug!("Capture group detected, 1st matching capture group will be returned");
            capture_group = 1;
        }

        let value = captures.get(capture_group).map_or("", |m| m.as_str());
        debug!("Value fetched using regex for {} : {}", pattern, value);
        Ok(String::from(value))
    }
}

fn extract<T: Extractor>(extractor: T, body: &str, map: &HashMap<String, String>, env_map: &mut HashMap<String, String>)
 -> Result<(), Box<dyn Error + 'static>> {
    for (k, v) in map {
        let extracted_value = extractor.extract(v,body)?;
        env_map.insert(k.to_owned(), extracted_value);
    }

    Ok(())
}

pub async fn process(response: Response, extractors: &[model::Extractor], env_map: &mut HashMap<String, String>) -> Result<(), Box<dyn Error + 'static>> {
    //run the extractors for headers first
    execute_header_extractors(response.headers(), extractors, env_map)?;

    //run body extractors
    execute_body_extractors(response, extractors, env_map).await?;

    Ok(())
}

fn execute_header_extractors(headers: &HeaderMap, extractors: &[model::Extractor], env_map: &mut HashMap<String, String>) 
-> Result<(), Box<dyn Error + 'static>> {
    for extractor in extractors {
        match extractor.from {
            model::ExtractFrom::Headers => {
                for (param,header_name) in &extractor.extract {
                    match headers.get(header_name) {
                        Some(header_value) => {
                            let header_value = header_value.to_str()?.to_owned();
                            debug!("Value fetched from header {} : {} ", header_name, header_value);
                            env_map.insert(param.to_owned(), header_value);
                        },
                        None => {
                            error!("No header found with name {} in the response", header_name);
                        }
                    }
                }
            },
            _ => ()
        }
    }

    Ok(())
}

async fn execute_body_extractors(response: Response, extractors: &[model::Extractor], env_map: &mut HashMap<String, String>)
-> Result<(), Box<dyn Error + 'static>> {

    let is_json_response = is_json_response(&response);
    let is_xml_response = !is_json_response && is_xml_response(&response);
    let body = get_response_as_string(response).await; 

    for extractor in extractors {
        match extractor.from {
            model::ExtractFrom::Body => {
                match extractor.extractor_type {
                    ExtractorType::GjsonPath  => {
                        if !is_json_response {
                            error!("Response is not in json format, json extractor will not be executed");
                            continue;
                        }
        
                        extract(JsonExtractor, &body, &extractor.extract, env_map)?; 
                    },
                    ExtractorType::Xpath => {
                        if !is_xml_response {
                            error!("Response is not in xml/html format, xml extractor will not be executed");
                            continue;
                        }
        
                        extract(XpathExtractor, &body, &extractor.extract, env_map)?; 
                    },
                    ExtractorType::RegEx => {
                        extract(RegExExtractor, &body, &extractor.extract, env_map)?; 
                    },
                    _ => {
                        error!("Invalid extractor type found to extract from body: {:?}", extractor.extractor_type)
                    }
                }  
            },
            _ => ()
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

#[cfg(test)]
mod tests {
    use crate::{parse::postprocessor::*};
    
    #[tokio::test]
    async fn test_is_json_response() {
        let response = reqwest::get("https://httpbin.org/get").await.unwrap();
        assert!(is_json_response(&response));
    }

    #[tokio::test]
    async fn test_is_not_json_response() {
        let response = reqwest::get("https://google.com/").await.unwrap();
        assert!(!is_json_response(&response));
    }

    #[tokio::test]
    async fn test_is_xml_response() {
        let response = reqwest::get("https://httpbin.org/xml").await.unwrap();
        assert!(is_xml_response(&response));
    }

    #[tokio::test]
    async fn test_is_html_response() {
        let response = reqwest::get("https://httpbin.org/html").await.unwrap();
        assert!(is_xml_response(&response));
    }

    #[tokio::test]
    async fn test_is_not_xml_response() {
        let response = reqwest::get("https://httpbin.org/get").await.unwrap();
        assert!(!is_xml_response(&response));
    }

    #[tokio::test]
    async fn test_json_extractor() {
        let response = reqwest::get("https://httpbin.org/get").await.unwrap();
        let response_as_str = &get_response_as_string(response).await;
        let json_extractor = JsonExtractor{};
        assert_eq!(json_extractor.extract("headers.Host", response_as_str).unwrap(), String::from("httpbin.org"));
    }

    #[tokio::test]
    async fn test_xml_extractor() {
        let response = reqwest::get("https://httpbin.org/xml").await.unwrap();
        let response_as_str = &get_response_as_string(response).await;
        let xml_extractor = XpathExtractor{};
        assert_eq!(xml_extractor.extract("//slide/title", response_as_str).unwrap(), String::from("Wake up to WonderWidgets!"));
        assert_eq!(xml_extractor.extract("//slide/@type", response_as_str).unwrap(), String::from("all"));
        assert_eq!(xml_extractor.extract("//slide[2]/item[3]/em[1]", response_as_str).unwrap(), String::from("buys"));
    }

    #[tokio::test]
    async fn test_html_extractor() {
        let response = reqwest::get("https://httpbin.org/html").await.unwrap();
        let response_as_str = &get_response_as_string(response).await;
        let xml_extractor = XpathExtractor{};
        assert_eq!(xml_extractor.extract("//h1", response_as_str).unwrap(), String::from("Herman Melville - Moby-Dick"));
        assert!(xml_extractor.extract("//body//p", response_as_str).unwrap().contains("summer-cool weather"));
    }

    #[tokio::test]
    async fn test_header_extractor() {
        let extractors = r#"
        - from: Headers
          extract:
            server: server   
        "#;

        let response = reqwest::get("https://httpbin.org/html").await.unwrap();
        let header_extractor = serde_yaml::from_str::<Vec<model::Extractor>>(extractors).unwrap();
        let mut env_map = HashMap::default();

        execute_header_extractors(response.headers(), &header_extractor, &mut env_map).unwrap();
        assert_eq!(env_map.len(), 1);
        assert_eq!(env_map.get("server").unwrap().as_str(), "gunicorn/19.9.0");
    }

    #[tokio::test]
    async fn test_regex_extractor_without_group() {
        let text = "my birthday is on 15-Mar";
        let regex_extractor = RegExExtractor{};
        assert_eq!(regex_extractor.extract(r"\d{2}-[A-Z]{1}[a-z]{2}", text).unwrap(), String::from("15-Mar"));
    }

    #[tokio::test]
    async fn test_regex_extractor_with_group() {
        let text = "Not my favorite movie: 'Citizen Kane' (1941).";
        let regex_extractor = RegExExtractor{};
        assert_eq!(regex_extractor.extract(r"'([^']+)'\s+\((\d{4})\)", text).unwrap(), String::from("Citizen Kane"));
    }

    #[tokio::test]
    async fn test_extractor() {
        let response = reqwest::get("https://httpbin.org/get").await.unwrap();
        let response_as_str = &get_response_as_string(response).await;
        let mut extractor = HashMap::default();
        extractor.insert(String::from("host"), String::from("headers.Host"));
        let mut env_map = HashMap::default();

        extract(JsonExtractor, response_as_str, &extractor, &mut env_map).unwrap();
        assert!(env_map.len() == 1);
        assert_eq!(env_map.get("host").unwrap(), "httpbin.org");
    }
}