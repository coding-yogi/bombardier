use crate::file;

use clap::{Arg, App, ArgMatches, SubCommand};
use log::{info, warn};
use serde::{Serialize, Deserialize, Deserializer, de::Error};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ExecConfig {

    #[serde(default)]
    pub environment_file: String,

    #[serde(deserialize_with = "check_json_file")]
    pub collection_file: String,

    #[serde(default = "default_report_file")]
    pub report_file: String,

    #[serde(default)]
    pub data_file: String,

    #[serde(deserialize_with = "check_non_zero")]
    #[serde(default = "default_to_one")]
    pub thread_count: u64,

    #[serde(default)]
    pub iterations: u64,

    #[serde(default)]
    pub execution_time: u64,

    #[serde(default = "default_to_one")]
    pub think_time: u64,

    #[serde(deserialize_with = "check_non_zero")]
    pub rampup_time: u64,
    
    #[serde(default)]
    pub handle_cookies: bool,

    #[serde(default)]
    pub continue_on_error: bool,

    #[serde(default)]
    pub log_to_file: bool,

    #[serde(default)]
    pub distributed: bool,

    #[serde(default)]
    pub nodes: Vec<String>,

    #[serde(default)]
    pub influxdb: InfluxDB,

    #[serde(default)]
    pub ssl: Ssl
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct InfluxDB {
    pub url: String,
    pub username: String,
    pub password: String,
    pub dbname: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Ssl {
    #[serde(default)]
    pub ignore_ssl: bool,

    #[serde(default)]
    pub accept_invalid_hostnames: bool,

    #[serde(deserialize_with = "check_der_or_pem")]
    pub certificate: String,

    #[serde(deserialize_with = "check_p12_or_pfx")]
    pub keystore: String,
    pub keystore_password: String,
}

const CONFIG_ARG_NAME: &str = "config json file";
const PORT_ARG_NAME: &str = "websocket port";
const JSON_EXT: &str = ".json";
pub const DER_EXT: &str = ".der";
pub const PEM_EXT: &str = ".pem";
pub const P12_EXT: &str = ".p12";
pub const PFX_EXT: &str = ".pfx";
const DEFAULT_REPORT_FILE: &str = "report.csv";

fn default_report_file() -> String {
    String::from(DEFAULT_REPORT_FILE)
}

fn default_to_one() -> u64 {
    1
}

pub fn create_cmd_app<'a, 'b>() -> App<'a, 'b> {
    let config_arg = create_config_arg(CONFIG_ARG_NAME);

    App::new("Bombardier")
        .version("0.1.0")
        .author("Coding Yogi <aniket.g2185@gmail.com>")
        .subcommand(SubCommand::with_name("bombard")
                .about("Executes the test")
                .arg(&config_arg))
        .subcommand(SubCommand::with_name("report")
                .about("Generates the report from report file")
                .arg(&config_arg))
        .subcommand(SubCommand::with_name("node")
                .about("Starts bombardier as a node")
                .arg(Arg::with_name(PORT_ARG_NAME)
                        .short("p")
                    .long("port")
                    .takes_value(true)
                    .required(true)
                    .validator(|s: String| {
                        match s.parse::<i32>() {
                            Ok(_) => Ok(()),
                            Err(_) => Err(String::from("Port should be an integer"))
                        }
                    })))
}

fn create_config_arg<'a>(arg_name: &'a str) -> Arg<'a, 'a> {
    Arg::with_name(arg_name)
        .short("c")
        .long("config")
        .takes_value(true)
        .required(true)
        .validator(|s: String| {
            match s.ends_with(JSON_EXT) {
                true => Ok(()),
                false => Err(String::from("File should be a .json file"))
            }
        })
        .display_order(0)
        .help("Execution configuration json file")
}

fn get_config_from_file(config_file_path: String) -> Result<ExecConfig, Box<dyn std::error::Error + Send + Sync>> {
    info!("Parsing config file {}", config_file_path);
    
    let content = file::get_content(&config_file_path)?;
    let config: ExecConfig = serde_json::from_str(&content)?;

    if config.execution_time == 0 && config.iterations == 0 {
        return Err("Both execution time and iterations cannot be 0".into());
    }

    if config.execution_time > 0 && config.iterations > 0 {
        warn!("Both execution time and iterations values provided. Execution time will be ignored");
    }

    Ok(config)
}

pub fn get_config(subcommand_args: Option<&ArgMatches<>>) -> Result<ExecConfig, Box<dyn std::error::Error + Send + Sync>> {
    let config_file_path = get_value_as_str(subcommand_args, CONFIG_ARG_NAME); 
    let config = get_config_from_file(config_file_path)?;
    Ok(config)
}

pub fn get_port(subcommand_args: Option<&ArgMatches<>>) -> i32 {
    get_value_as_int(subcommand_args, PORT_ARG_NAME)
}

fn check_non_zero <'de, D>(deserializer: D) -> Result<u64, D::Error> 
where D: Deserializer<'de> {    
    let val = u64::deserialize(deserializer)?;
    if val == 0 {
        return Err(Error::custom("Value cannot be zero"))
    }

    Ok(val)
}

fn check_der_or_pem <'de, D>(deserializer: D) -> Result<String, D::Error> 
where D: Deserializer<'de> {   
    let val = String::deserialize(deserializer)?;
    if val != "" && !(val.ends_with(DER_EXT) || val.ends_with(PEM_EXT)){
        return Err(Error::custom("File should be a .pem or .der file"))
    }

    Ok(val)
}

fn check_p12_or_pfx <'de, D>(deserializer: D) -> Result<String, D::Error> 
where D: Deserializer<'de> {   
    let val = String::deserialize(deserializer)?;
    if val != "" && !(val.ends_with(P12_EXT) || val.ends_with(PFX_EXT)){
        return Err(Error::custom("File should be a .p12 or .pfx file"))
    }

    Ok(val)
}

fn check_json_file <'de, D>(deserializer: D) -> Result<String, D::Error> 
where D: Deserializer<'de> { 
    let val = String::deserialize(deserializer)?;
    if !val.ends_with(JSON_EXT)  {
        return Err(Error::custom("File should be a .json file"))
    }

    Ok(val)
}

fn get_value_as_str(matches: Option<&ArgMatches>, arg: &str) -> String {
    match matches {
        Some(x) => match x.value_of(arg) {
                        Some(y) => y.to_string(),
                        None => "".to_string()
        },
        None => "".to_string()
    }
}

fn get_value_as_int(matches: Option<&ArgMatches>, arg: &str) -> i32 {
    match matches {
        Some(x) => match x.value_of(arg) {
                        Some(y) => y.parse::<i32>().unwrap(),
                        None => 0
        },
        None => 0
    }
}