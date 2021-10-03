use log::{error, info, warn};

use std::{
    collections::HashMap,
    error::Error
};

use crate::{
    model::{Environment, Config, Request, Root}, 
    parse::preprocessor
};

pub fn parse_config(content: String) -> Result<Config, Box<dyn std::error::Error>> {
    let config: Config = match serde_yaml::from_str(&content) {
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

pub fn parse_requests(content: String, env_map: &HashMap<String, String>) -> Result<Vec<Request>, Box<dyn Error>> {
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
    let mut env_map: HashMap<String, String> = HashMap::with_capacity(30);

    if content.is_empty() {
        warn!("No environments data is being used for execution");
        return Ok(env_map);
    }

    info!("Parsing env map");
    let env: Environment = match serde_yaml::from_str(content) {
        Ok(e) => e,
        Err(err) => {
            error!("Parsing env content failed: {}", err.to_string());
            return Err(err.into())
        }
    };

    for var in env.variables {
        let key = var.0.as_str().unwrap().to_string();
        let value = var.1.as_str().unwrap().to_string();
        env_map.insert(key, value);
    }

    Ok(env_map)
}

fn param_substitution_required(request: &Request) -> bool {
    let request_string = serde_yaml::to_string(request).unwrap();
    request_string.contains("{{")
}