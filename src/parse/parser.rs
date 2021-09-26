use log::{error,warn};

use crate::cmd;

pub fn parse_config_from_string(content: String) -> Result<cmd::ExecConfig, Box<dyn std::error::Error>> {
    let config: cmd::ExecConfig = match serde_yaml::from_str(&content) {
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