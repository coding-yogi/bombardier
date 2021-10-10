use log::{error, warn};
use lazy_static::lazy_static;
use regex::Regex;
use rustc_hash::FxHashMap as HashMap;

use crate::model::Request;

pub fn process(request: &Request, env_map: &HashMap<String, String>) -> Request {
    if !env_map.is_empty() {
        let s_request = serde_json::to_string(&request).expect("Request cannot be serialized");
        let s_request = param_substitution(&s_request, env_map);

        if let Ok(new_request) = serde_json::from_str::<Request>(&s_request) {
            return new_request;
        } else {
            error!("Unable to deserialize request object after parameter replacement. Returning original request");
        }
    }

    request.to_owned()
}

pub fn param_substitution(content: &str, params: &HashMap<String, String>) -> String {
   let mut new_value = String::from(content);

    lazy_static! {
        static ref RE: Regex = Regex::new(r"\{\{(\w+)\}\}").unwrap();
    }

    if content.contains("{{") {
        for cap in RE.captures_iter(content) {
            if let Some(map_value) = params.get(&cap[1]) {
                new_value = new_value.replace(&cap[0], map_value);
            } else {
                warn!("Param {} not found in env map", &cap[1]);
            }
        }
    }

    new_value
}

#[test]
fn test_param_substitution_with_no_sustitutions() {
    let scenarios_yaml = r"
    version: 1.0
    scenarios:
    - name: scenario1
      requests:
      - name: echoGet
        method: GET
        url: 'https://google.com/'
        extractors:
        - type: gjsonpath
          extract:
            authHeader: 'headers.authorization'
            host: 'headers.host'";

    let mut env_map = HashMap::default();
    env_map.insert(String::from("url"), String::from("https://google.com"));

    let substituted_string = param_substitution(scenarios_yaml, &env_map);
    assert_eq!(String::from(scenarios_yaml), substituted_string);
}

#[test]
fn test_param_substitution_with_multiple_sustitutions() {
    let scenarios_yaml = r"
    version: 1.0
    scenarios:
    - name: scenario1
      requests:
      - name: echoGet
        method: {{method}}
        url: '{{baseurl}}'
        headers:
          authorization: 'jwt {{token}}'
        body:
          urlencoded:
            param1: '{{param1Value}}'
            param2: '{{param2Value}}'";

    let mut env_map = HashMap::default();
    env_map.insert(String::from("method"), String::from("POST"));
    env_map.insert(String::from("baseurl"), String::from("https://google.com"));
    env_map.insert(String::from("token"), String::from("some_token_value"));
    env_map.insert(String::from("param1Value"), String::from("value1"));
    env_map.insert(String::from("param2Value"), String::from("value2"));

    let expected_substituted_yaml = r"
    version: 1.0
    scenarios:
    - name: scenario1
      requests:
      - name: echoGet
        method: POST
        url: 'https://google.com'
        headers:
          authorization: 'jwt some_token_value'
        body:
          urlencoded:
            param1: 'value1'
            param2: 'value2'";

    let substituted_string = param_substitution(scenarios_yaml, &env_map);
    assert_eq!(String::from(substituted_string), String::from(expected_substituted_yaml));
}

#[test]
fn test_param_substitution_with_missing_sustitutions() {
    let scenarios_yaml = r"
    version: 1.0
    scenarios:
    - name: scenario1
      requests:
      - name: echoGet
        method: {{method}}
        url: '{{baseurl}}'";

    let mut env_map = HashMap::default();
    env_map.insert(String::from("baseurl"), String::from("https://google.com"));

    let expected_substituted_yaml = r"
    version: 1.0
    scenarios:
    - name: scenario1
      requests:
      - name: echoGet
        method: {{method}}
        url: 'https://google.com'";

    let substituted_string = param_substitution(scenarios_yaml, &env_map);
    assert_eq!(String::from(substituted_string), String::from(expected_substituted_yaml));
}

#[test]
fn test_process() {
    let scenarios_yaml = r"
    version: 1.0
    scenarios:
    - name: scenario1
      requests:
      - name: echoGet
        method: {{method}}
        url: '{{baseurl}}'";

    let mut env_map = HashMap::default();
    env_map.insert(String::from("method"), String::from("POST"));
    env_map.insert(String::from("baseurl"), String::from("https://google.com"));

    let request = serde_yaml::from_str::<Request>(scenarios_yaml).unwrap();
    let processed_request = process(&request, &env_map);

    assert_eq!(processed_request.method, String::from("POST"));
    assert_eq!(processed_request.url, String::from("https://google.com"))   
}