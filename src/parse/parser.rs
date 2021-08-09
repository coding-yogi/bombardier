use log::{error, warn};

use std::{
    collections::HashMap,
    error::Error,
};

use crate::{file, model::scenarios};

pub fn parse_requests(content: String, env_map: &HashMap<String, String>) -> Result<Vec<scenarios::Request>, Box<dyn Error>> {
    let scenarios_yml = file::param_substitution(content, &env_map);

    let root: scenarios::Root = match serde_yaml::from_str(&scenarios_yml) {
        Ok(r) => r,
        Err(err) => {
            error!("error deserializing yaml: {}", err.to_string());
            return Err("error while deserailizing scnarios yaml".into());
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

pub fn get_env_map(env_file: &str) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut env_map: HashMap<String, String> = HashMap::new();

    //Check if environment file is specified
    if env_file == "" {
        warn!("No environment json file specified in config");
    } else {
        let env_file_content = file::get_content(env_file)?;
        let env: scenarios::Environment = serde_yaml::from_str(&env_file_content)?;
    
        for var in env.variables {
            let key = var.0.as_str().unwrap().to_string();
            let value = var.1.as_str().unwrap().to_string();
            env_map.insert(key, value);
        }
    }

    Ok(env_map)
}

pub fn get_vec_data_map(data_file: &str) -> Result<Vec<HashMap<String, String>>, csv::Error> {
    let mut vec_data_map: Vec<HashMap<String, String>> = Vec::new();

    if data_file != "" {
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .trim(csv::Trim::All)
            .from_path(data_file)?;

        let mut records_iterator = reader.records();

        let headers: Vec<String> = records_iterator
            .next()
            .unwrap()?
            .iter()
            .map(|s| s.to_string())
            .collect();


        vec_data_map = records_iterator.map(|record| {
            headers.iter()
                .zip(record.unwrap().iter())
                .map(|(k,v)| (k.clone(), v.to_string()))
                .collect()
        }).collect();
    }
    
    Ok(vec_data_map)
}