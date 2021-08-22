use log::warn;

use std::{
    collections::HashMap,
    error::Error,
};

use crate::{cmd, file, model::*, report::csv};

pub fn parse_config_from_string(content: String) -> Result<cmd::ExecConfig, Box<dyn std::error::Error>> {
    let config: cmd::ExecConfig = serde_json::from_str(&content)?;

    if config.execution_time == 0 && config.iterations == 0 {
        return Err("Both execution time and iterations cannot be 0".into());
    }

    if config.execution_time > 0 && config.iterations > 0 {
        warn!("Both execution time and iterations values provided. Execution time will be ignored");
    }

    Ok(config)
}

pub fn parse_requests(content: String, env_map: &HashMap<String, String>) -> Result<Vec<Request>, Box<dyn Error>> {
    let scenarios_yml = file::param_substitution(content, &env_map);

    let root: Root = serde_yaml::from_str(&scenarios_yml)?;

    let mut requests = Vec::<Request>::new();
  
    for scenario in root.scenarios {
        for request in scenario.requests {
            requests.push(request);
        }
    } 

    Ok(requests)
}

pub fn get_env_map(content: &str) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let env: Environment = serde_yaml::from_str(content)?;
    let mut env_map: HashMap<String, String> = HashMap::with_capacity(30);

    for var in env.variables {
        let key = var.0.as_str().unwrap().to_string();
        let value = var.1.as_str().unwrap().to_string();
        env_map.insert(key, value);
    }

    Ok(env_map)
}

pub async fn get_vec_data_map(data_content: String) -> Result<Vec<HashMap<String, String>>, Box<dyn Error>> {
    if data_content == "" {
        return Ok(Vec::<HashMap<String, String>>::new())
    }
    
    let vec_data_map = 
        csv::CSVReader.get_records(data_content.as_bytes()).await;

    match vec_data_map {
        Ok(v) => Ok(v),
        Err(err) => Err(err.into())
    }
}