use csv_async;
use log::{error, warn};
use futures::StreamExt;

use std::{
    collections::HashMap,
    error::Error,
};

use crate::{cmd, file, model::scenarios};

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

pub fn parse_requests(content: String, env_map: &HashMap<String, String>) -> Result<Vec<scenarios::Request>, Box<dyn Error>> {
    let scenarios_yml = file::param_substitution(content, &env_map);

    let root: scenarios::Root = match serde_yaml::from_str(&scenarios_yml) {
        Ok(r) => r,
        Err(err) => {
            error!("Error deserializing yaml: {}", err.to_string());
            return Err("Error while deserailizing scenarios yaml".into());
        }
    };

    let mut requests = Vec::<scenarios::Request>::new();
  
    for scenario in root.scenarios {
        for request in scenario.requests {
            requests.push(request);
        }
    } 

    Ok(requests)
}

pub fn get_env_map(content: &str) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut env_map: HashMap<String, String> = HashMap::new();
    let env: scenarios::Environment = serde_yaml::from_str(content)?;

    for var in env.variables {
        let key = var.0.as_str().unwrap().to_string();
        let value = var.1.as_str().unwrap().to_string();
        env_map.insert(key, value);
    }

    Ok(env_map)
}

pub async fn get_vec_data_map(data_content: String) -> Result<Vec<HashMap<String, String>>, csv_async::Error> {

    if data_content == "" {
        return Ok(Vec::<HashMap<String, String>>::new())
    }

    let mut reader = csv_async::AsyncReaderBuilder::new()
        .has_headers(false)
        .trim(csv_async::Trim::All)
        .create_reader(data_content.as_bytes());

    let mut records_iterator = reader.records();

    let headers= match records_iterator.next().await {
        Some(item) => {
            match item {
                Ok(item) => item.iter()
                .map(|s| s.to_owned())
                .collect(),
                Err(err) => return Err(err)
            }
        },
        None => Vec::new()
    };

    let vec_data_map = records_iterator.map(|record| {
        headers.iter()
            .zip(record.unwrap().iter())
            .map(|(k,v)| (k.to_owned(), v.to_owned()))
            .collect::<HashMap<String, String>>()
    }).collect::<Vec<HashMap<String, String>>>().await;
    
    Ok(vec_data_map)
}