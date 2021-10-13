use log::{error, info, warn};

use std::{
    error::Error
};

use rustc_hash::FxHashMap as HashMap;

use crate::{
    model::{Environment, Config, Request, Root}, 
    parse::preprocessor
};

pub fn parse_config(content: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let config: Config = match serde_yaml::from_str(content) {
        Ok(c) => c,
        Err(err) => {
            error!("Error while parsing config: {}", err.to_string());
            return Err(err.into())
        }
    };

    if config.execution_time == 0 && config.iterations == 0 {
        return Err("Both execution time and iterations cannot be 0".into());
    }

    if config.execution_time > 0 && config.iterations > 0 {
        warn!("Both execution time and iterations values provided. Execution time will be ignored");
    }

    Ok(config)
}

pub fn parse_requests(content: &str, env_map: &HashMap<String, String>) -> Result<Vec<Request>, Box<dyn Error>> {
    info!("Preparing bombardier requests");
    let scenarios_yml = preprocessor::param_substitution(content, env_map);

    let root: Root = match serde_yaml::from_str(&scenarios_yml) {
        Ok(r) => r,
        Err(err) => {
            error!("Parsing bombardier requests failed: {}", err.to_string());
            return Err(err.into())
        }
    };

    let mut requests = Vec::<Request>::new();
  
    for scenario in root.scenarios {
        for mut request in scenario.requests {
            request.id = uuid::Uuid::new_v4();
            request.requires_preprocessing = param_substitution_required(&request);
            requests.push(request);
        }
    } 

    Ok(requests)
}

pub fn parse_env_map(content: &str) -> Result<HashMap<String, String>, Box<dyn Error>> {
    if content.is_empty() {
        warn!("No environments data is being used for execution");
        return Ok(HashMap::default());
    }

    info!("Parsing env map");
    let env: Environment = match serde_yaml::from_str(content) {
        Ok(e) => e,
        Err(err) => {
            error!("Parsing env content failed: {}", err.to_string());
            return Err(err.into())
        }
    };

    Ok(env.variables.iter()
        .map(|var| (var.0.as_str().unwrap().to_string(), var.1.as_str().unwrap().to_string()))
        .collect::<HashMap<String, String>>())
}

fn param_substitution_required(request: &Request) -> bool {
    let request_string = serde_yaml::to_string(request).unwrap();
    request_string.contains("{{")
}

#[cfg(test)]
mod tests {
    use crate::{model::{ExtractFrom, ExtractorType}, parser::*};

    #[test]
    fn test_parse_config() {
        let config_yaml = r"
        version: 1.0
        threadCount: 100
        iterations: 0
        executionTime: 300
        rampUpTime: 100
        continueOnError: true
        handleCookies: false";
    
        let config_result = parse_config(config_yaml);
        assert!(config_result.is_ok());
    
        //Asserts
        let config = config_result.unwrap();
        assert_eq!(config.thread_count, 100);
        assert_eq!(config.iterations, 0);
        assert_eq!(config.execution_time, 300);
        assert_eq!(config.rampup_time, 100);
        assert_eq!(config.continue_on_error, true);
        assert_eq!(config.handle_cookies, false);
    }
    
    #[test]
    fn test_error_for_exec_time_and_iterations_as_zero() {
        let config_yaml = r"
        version: 1.0
        threadCount: 100
        iterations: 0
        executionTime: 0
        rampUpTime: 100
        continueOnError: true
        handleCookies: false";
    
        let config_result = parse_config(config_yaml);
        assert!(config_result.is_err());
        assert!(config_result.err().unwrap().to_string().contains("Both execution time and iterations cannot be 0"));
    }
    
    #[test]
    fn test_defaults_for_missing_values() {
        let config_yaml = r"
        version: 1.0
        iterations: 1";
    
        let config_result = parse_config(config_yaml);
        assert!(config_result.is_ok());
    
        //assert defaults
        let config = config_result.unwrap();
        assert_eq!(config.thread_count, 1);
        assert_eq!(config.iterations, 1);
        assert_eq!(config.execution_time, 0);
        assert_eq!(config.rampup_time, 1);
        assert_eq!(config.continue_on_error, false);
        assert_eq!(config.handle_cookies, false);
    }
    
    #[test]
    fn test_parse_env_map() {
        let env_map_yaml = r"
        version: 1.0,
        variables:
          variable1: value1
          variable2: value2";
    
        let env_map = parse_env_map(env_map_yaml);
        assert!(env_map.is_ok());
    
        let env_map = env_map.unwrap();
        assert_eq!(env_map.len(), 2);
        assert_eq!(env_map.get("variable1").unwrap(), "value1");
    }
    
    #[test]
    fn test_error_for_invalid_env_map_yaml() {
        let env_map_yaml = r"
        version: 1.0";
    
        let env_map = parse_env_map(env_map_yaml);
        assert!(env_map.is_err());
        assert!(env_map.err().unwrap().to_string().contains("missing field `variables`"));
    
        let env_map_yaml = r"
        version: 1.0
        variables:";
    
        let env_map = parse_env_map(env_map_yaml);
        assert!(env_map.is_err());
        assert!(env_map.err().unwrap().to_string().contains("variables: invalid type: unit value, expected a YAML mapping"));
    }
    
    #[test]
    fn test_empty_content_in_env_map_yaml() {
        let env_map_yaml = r"";
    
        let env_map = parse_env_map(env_map_yaml);
        assert!(env_map.is_ok());
        assert_eq!(env_map.unwrap().len(), 0);
    }
    
    #[test]
    fn test_parse_request_with_no_substitution() {
        let scenarios_yaml = r"
        version: 1.0
        scenarios:
        - name: scenario1
          requests:
          - name: echoGet
            method: GET
            url: 'https://google.com/'
            extractors:
            - type: GjsonPath
              extract:
                authHeader: 'headers.authorization'
                host: 'headers.host'";
    
        let scenarios = parse_requests(scenarios_yaml, &HashMap::default());
        assert!(scenarios.is_ok());
    
        let requests = scenarios.unwrap();
        assert_eq!(requests.len(),1);
        assert_eq!(requests[0].name, "echoGet");
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].url, "https://google.com/");
        assert_eq!(requests[0].extractors.len(), 1);
        assert_eq!(requests[0].extractors[0].extractor_type, ExtractorType::GjsonPath);
        assert_eq!(requests[0].extractors[0].extract.len(), 2);
        assert_eq!(requests[0].requires_preprocessing, false); //false as no more substitution required
    }
    
    #[test]
    fn test_parse_request_with_substitution() {
        let scenarios_yaml = r"
        version: 1.0
        scenarios:
        - name: scenario1
          requests:
          - name: echoGet
            method: GET
            url: '{{baseUrl}}'
            headers:
              authorization: 'jwt {{token}}'
            body:
              urlencoded:
                param1: '{{param1Value}}'
                param2: '{{param2Value}}'";
    
        let env_map_yaml = r"
        version: 1.0,
        variables:
            baseUrl: 'https://google.com/'
            token: some_token_value
            param1Value: param1_value";
    
        let env_map = parse_env_map(env_map_yaml).unwrap();
        let requests = parse_requests(scenarios_yaml, &env_map).unwrap();
    
        assert_eq!(requests[0].name, "echoGet");
        assert_eq!(requests[0].method, "GET");
        assert_eq!(requests[0].url, "https://google.com/");
        assert_eq!(requests[0].headers.get(&serde_yaml::Value::from("authorization")).unwrap(), "jwt some_token_value");
        assert_eq!(requests[0].body.urlencoded.get(&serde_yaml::Value::from("param1")).unwrap(), "param1_value");
        assert_eq!(requests[0].requires_preprocessing, true); //true as {{param2Value}} was not part of env map
    }
    
    #[test]
    fn test_error_for_missing_request_url() {
        let scenarios_yaml = r"
        version: 1.0
        scenarios:
        - name: scenario1
          requests:
          - name: echoGet
          - method: GET";
    
        let requests = parse_requests(scenarios_yaml, &HashMap::default());
        assert!(requests.is_err());
        assert!(requests.err().unwrap().to_string().contains("missing field `url`"));
    }
    
    #[test]
    fn test_error_for_missing_request_name() {
        let scenarios_yaml = r"
        version: 1.0
        scenarios:
        - name: scenario1
          requests:
          - url: 'http://google.com/'
            method: GET";
    
        let requests = parse_requests(scenarios_yaml, &HashMap::default());
        assert!(requests.is_err());
        assert!(requests.err().unwrap().to_string().contains("missing field `name`"));
    }
    
    #[test]
    fn test_error_for_missing_request_method() {
        let scenarios_yaml = r"
        version: 1.0
        scenarios:
        - name: scenario1
          requests:
          - name: echoGet
            url: 'http://google.com/'";
    
        let requests = parse_requests(scenarios_yaml, &HashMap::default());
        assert!(requests.is_err());
        assert!(requests.err().unwrap().to_string().contains("missing field `method`"));
    }
    
    #[test]
    fn test_for_parsing_raw_body() {
        let scenarios_yaml = r#"
        version: 1.0
        scenarios:
        - name: scenario1
          requests:
          - name: echoGet
            method: POST
            url: 'http://google.com'
            headers:
              content-type: application/json
            body:
              raw: '{"test": "test"}'
        "#;
        
        let requests = parse_requests(scenarios_yaml, &HashMap::default()).unwrap();
        assert_eq!(requests[0].body.raw,String::from(r#"{"test": "test"}"#));
    }
    
    
    #[test]
    fn test_for_parsing_multipart_form_body() {
        let scenarios_yaml = r#"
        version: 1.0
        scenarios:
        - name: scenario1
          requests:
          - name: echoGet
            method: POST
            url: 'http://google.com'
            headers:
              content-type: application/json
            body:
              formdata:
                key21: value21
                key22: value22
                _file: '/Users/aniket.gadre/work/repos/bombardier/examples/configdev.json'
        "#;
        
        let requests = parse_requests(scenarios_yaml, &HashMap::default()).unwrap();
        assert_eq!(requests[0].body.formdata.len(),3);
    }

    #[test]
    fn test_error_for_invalid_extractor_type() {
        let scenarios_yaml = r"
        version: 1.0
        scenarios:
        - name: scenario1
          requests:
          - name: echoGet
            method: GET
            url: 'https://google.com/'
            extractors:
            - type: jsonPath
              from: Body
              extract:
                authHeader: 'headers.authorization'
                host: 'headers.host'";

        let requests = parse_requests(scenarios_yaml, &HashMap::default());
        assert!(requests.is_err());
        assert!(requests.err().unwrap().to_string().contains("expected one of `GjsonPath`, `Xpath`, `RegEx`, `None`"));        
    }

    #[test]
    fn test_error_for_invalid_extractor_from() {
        let scenarios_yaml = r"
        version: 1.0
        scenarios:
        - name: scenario1
          requests:
          - name: echoGet
            method: GET
            url: 'https://google.com/'
            extractors:
            - type: GjsonPath
              from: Variable
              extract:
                authHeader: 'headers.authorization'
                host: 'headers.host'";

        let requests = parse_requests(scenarios_yaml, &HashMap::default());
        assert!(requests.is_err());
        assert!(requests.err().unwrap().to_string().contains("expected `Body` or `Headers`"));        
    }

    #[test]
    fn test_extract_from_defaults_to_body() {
        let scenarios_yaml = r"
        version: 1.0
        scenarios:
        - name: scenario1
          requests:
          - name: echoGet
            method: GET
            url: 'https://google.com/'
            extractors:
            - type: GjsonPath
              extract:
                authHeader: 'headers.authorization'";

        let requests = parse_requests(scenarios_yaml, &HashMap::default()).unwrap();
        assert_eq!(requests[0].extractors[0].from, ExtractFrom::Body);        
    }

    #[test]
    fn test_extract_type_defaults_to_none_for_headers() {
        let scenarios_yaml = r"
        version: 1.0
        scenarios:
        - name: scenario1
          requests:
          - name: echoGet
            method: GET
            url: 'https://google.com/'
            extractors:
            - from: Headers
              extract:
                server: server";

        let requests = parse_requests(scenarios_yaml, &HashMap::default()).unwrap();
        assert_eq!(requests[0].extractors[0].extractor_type, ExtractorType::None);    
    }
}