use clap::{Arg, App, ArgMatches, SubCommand};
use log::{info, warn};
use serde::{Serialize, Deserialize, Deserializer, de::Error};

use crate::file;

//ExecConfig is the model for execution configuration
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ExecConfig {

    #[serde(deserialize_with = "check_yml_file")]
    pub scenarios_file: String,

    #[serde(deserialize_with = "check_yml_file")]
    pub environment_file: String,

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

    #[serde(default, deserialize_with = "check_der_or_pem")]
    pub certificate: String,

    #[serde(default, deserialize_with = "check_p12_or_pfx")]
    pub keystore: String,

    #[serde(default)]
    pub keystore_password: String,
}

const CONFIG_ARG_NAME: &str = "config json file";
pub const SERVER_PORT_ARG_NAME: &str = "server port";
pub const SOCKET_PORT_ARG_NAME: &str = "websocket port";
const REPORT_FILE_ARG_NAME: &str = "report file";
const JSON_EXT: &str = ".json";
const CSV_EXT: &str = ".csv";
const YAML_EXT: &str = ".yml";
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
    App::new("Bombardier")
        .version("0.1.0")
        .author("Coding Yogi <aniket.g2185@gmail.com>")
        .subcommand(SubCommand::with_name("bombard")
                .about("Executes the test")
                .arg(Arg::with_name(CONFIG_ARG_NAME)
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
                    .help("Execution configuration json file")))
        .subcommand(SubCommand::with_name("report")
                .about("Generates the report from report file")
                .arg(Arg::with_name(REPORT_FILE_ARG_NAME)
                    .short("f")
                    .long("report_file")
                    .takes_value(true)
                    .required(true)
                    .validator(|s: String| {
                        match s.ends_with(CSV_EXT) {
                            true => Ok(()),
                            false => Err(String::from("File should be a .csv file"))
                        }
                    })
                    .help("Report file in csv format")))
        .subcommand(SubCommand::with_name("node")
                .about("Starts bombardier as a node")
                .arg(Arg::with_name(SOCKET_PORT_ARG_NAME)
                    .short("p")
                    .long("port")
                    .takes_value(true)
                    .required(true)
                    .validator(|s: String| {
                        match s.parse::<i32>() {
                            Ok(_) => Ok(()),
                            Err(_) => Err(String::from("Port should be an integer"))
                        }
                    }
                )
            )
        )
        .subcommand(SubCommand::with_name("serve")
                .about("Starts bombardier as a web server")
                .args(&[
                    Arg::with_name(SERVER_PORT_ARG_NAME)
                    .short("p")
                    .long("port")
                    .takes_value(true)
                    .required(true)
                    .validator(|s: String| {
                        match s.parse::<u16>() {
                            Ok(_) => Ok(()),
                            Err(_) => Err(String::from("Port should be an integer"))
                        }
                    }),
                    Arg::with_name(SOCKET_PORT_ARG_NAME)
                    .short("s")
                    .long("socket_port")
                    .takes_value(true)
                    .required(true)
                    .validator(|s: String| {
                        match s.parse::<u16>() {
                            Ok(_) => Ok(()),
                            Err(_) => Err(String::from("Socket port should be an integer"))
                        }
                    })
                ])
        )
}

fn get_config_from_file(config_file_path: String) -> Result<ExecConfig, Box<dyn std::error::Error>> {
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

pub fn get_config(subcommand_args: Option<&ArgMatches<>>) -> Result<ExecConfig, Box<dyn std::error::Error>> {
    let config_file_path = arg_value_as_str(subcommand_args, CONFIG_ARG_NAME); 
    let config = get_config_from_file(config_file_path)?;
    Ok(config)
}

pub fn get_port(subcommand_args: Option<&ArgMatches<>>, name: &str) -> u16 {
    arg_value_as_u16(subcommand_args, name)
}

pub fn get_report_file(subcommand_args: Option<&ArgMatches<>>) -> String {
    arg_value_as_str(subcommand_args, REPORT_FILE_ARG_NAME)
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

fn check_yml_file <'de, D>(deserializer: D) -> Result<String, D::Error> 
where D: Deserializer<'de> { 
    let val = String::deserialize(deserializer)?;
    if !val.ends_with(YAML_EXT)  {
        return Err(Error::custom("File should be a .yml file"))
    }

    Ok(val)
}

fn arg_value_as_str(matches: Option<&ArgMatches>, arg: &str) -> String {
    match matches {
        Some(x) => match x.value_of(arg) {
                        Some(y) => y.to_string(),
                        None => "".to_string()
        },
        None => "".to_string()
    }
}

fn arg_value_as_u16(matches: Option<&ArgMatches>, arg: &str) -> u16 {
    match matches {
        Some(x) => match x.value_of(arg) {
                        Some(y) => y.parse::<u16>().unwrap(),
                        None => 0
        },
        None => 0
    }
}