use log::warn;

use crate::cmd;

pub fn parse_config_from_string(content: String) -> Result<cmd::ExecConfig, Box<dyn std::error::Error>> {
    let config: cmd::ExecConfig = serde_yaml::from_str(&content)?;

    if config.execution_time == 0 && config.iterations == 0 {
        return Err("Both execution time and iterations cannot be 0".into());
    }

    if config.execution_time > 0 && config.iterations > 0 {
        warn!("Both execution time and iterations values provided. Execution time will be ignored");
    }

    Ok(config)
}