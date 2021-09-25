use clap::{Arg, App, ArgMatches, SubCommand};
use serde::{Serialize, Deserialize, Deserializer, de::Error};

//ExecConfig is the model for execution configuration
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ExecConfig {
    #[serde(deserialize_with = "check_non_zero")]
    #[serde(default = "default_to_one")]
    #[serde(rename = "threadCount")]
    pub thread_count: u64,

    #[serde(default)]
    pub iterations: u64,

    #[serde(default)]
    #[serde(rename = "executionTime")]
    pub execution_time: u64,

    #[serde(default = "default_to_one")]
    #[serde(rename = "thinkTime")]
    pub think_time: u64,

    #[serde(deserialize_with = "check_non_zero")]
    #[serde(rename = "rampUpTime")]
    pub rampup_time: u64,
    
    #[serde(default)]
    #[serde(rename = "handleCookies")]
    pub handle_cookies: bool,

    #[serde(default)]
    pub distributed: bool,

    #[serde(default)]
    #[serde(rename = "continueOnError")]
    pub continue_on_error: bool,

    #[serde(default)]
    pub database: Database,

    #[serde(default)]
    pub ssl: Ssl
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Database {
    #[serde(rename = "type")]
    #[serde(default)]
    pub db_type: String,

    #[serde(default)]
    pub url: String,

    #[serde(default)]
    pub host: String,

    #[serde(default)]
    pub port: String,

    #[serde(default)]
    pub user: String,

    #[serde(default)]
    pub password: String,

    #[serde(default)]
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Ssl {
    #[serde(default)]
    #[serde(rename = "ignoreSSL")]
    pub ignore_ssl: bool,

    #[serde(default)]
    #[serde(rename = "acceptInvalidHostnames")]
    pub accept_invalid_hostnames: bool,

    #[serde(default, deserialize_with = "check_der_or_pem")]
    pub certificate: String,

    #[serde(default, deserialize_with = "check_p12_or_pfx")]
    pub keystore: String,

    #[serde(default)]
    #[serde(rename = "keystorePassword")]
    pub keystore_password: String,
}

//Files
pub const CONFIG_FILE_ARG_NAME: &str = "config yml file";
pub const SCENARIOS_FILE_ARG_NAME: &str = "scenarios yml file";
pub const ENVIRONMENT_FILE_ARG_NAME: &str = "environments yml file";
pub const DATA_FILE_ARG_NAME: &str = "data csv file";
pub const REPORT_FILE_ARG_NAME: &str = "report file";

//Hub
pub const HUB_ADDRESS_ARG_NAME: &str = "hub adress as <ip>::<port>";

//Ports
pub const SERVER_PORT_ARG_NAME: &str = "server port";
pub const SOCKET_PORT_ARG_NAME: &str = "websocket port";

//File extensions
const CSV_EXT: &str = ".csv";
const YAML_EXT: &str = ".yml";
pub const DER_EXT: &str = ".der";
pub const PEM_EXT: &str = ".pem";
pub const P12_EXT: &str = ".p12";
pub const PFX_EXT: &str = ".pfx";

pub const DEFAULT_REPORT_FILE: &str = "report.csv";

fn get_arg(name: &'static str, short:&str, required: bool, help: &'static str) -> Arg<'static, 'static> {
    Arg::with_name(name)
        .short(short)
        .takes_value(true)
        .required(required)
        .help(help)
}

fn default_to_one() -> u64 {
    1
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

pub fn create_cmd_app<'a, 'b>() -> App<'a, 'b> {
    let is_yaml = |s: String| {
        match s.ends_with(YAML_EXT) {
            true => Ok(()),
            false => Err(String::from("File should be a .yml file"))
        }
    };

    let is_csv = |s: String| {
        match s.ends_with(CSV_EXT) {
            true => Ok(()),
            false => Err(String::from("File should be a .csv file"))
        }
    };

    let is_int = |s: String| {
        match s.parse::<u16>() {
            Ok(_) => Ok(()),
            Err(_) => Err(String::from("should be an integer"))
        }
    };

    App::new("Bombardier")
        .version("0.1.0")
        .author("Coding Yogi <aniket.g2185@gmail.com>")
        .subcommand(SubCommand::with_name("bombard")
                .about("Executes the test")
                .args(&[
                    get_arg(&CONFIG_FILE_ARG_NAME, "c", true, "Execution config yml file")
                    .validator(is_yaml)
                    .display_order(0),

                    get_arg(&SCENARIOS_FILE_ARG_NAME, "s", true, "Scenarios yml file")
                    .validator(is_yaml)
                    .display_order(1),

                    get_arg(&ENVIRONMENT_FILE_ARG_NAME, "e", false, "Environment yml file")
                    .validator(is_yaml)
                    .display_order(2),

                    get_arg(&DATA_FILE_ARG_NAME, "d", false, "Data csv file")
                    .validator(is_csv)
                    .display_order(3),

                    get_arg(&REPORT_FILE_ARG_NAME, "r", false, "report csv file")
                    .validator(is_csv)
                    .display_order(4),
                ]))

        .subcommand(SubCommand::with_name("report")
                .about("Generates the report from report file")
                .arg(get_arg(&REPORT_FILE_ARG_NAME, "r", true, "report file")
                .validator(is_csv)))

        .subcommand(SubCommand::with_name("node")
                .about("Starts bombardier as a node")
                .arg(get_arg(&HUB_ADDRESS_ARG_NAME, "h", true, "hub address <ip>:<port>")))

        .subcommand(SubCommand::with_name("hub")
                .about("Starts bombardier as a hub server")
                .args(&[
                    get_arg(&SERVER_PORT_ARG_NAME, "p", true, "rest server port")
                    .validator(is_int),

                    get_arg(&SOCKET_PORT_ARG_NAME, "s", true, "socket server port")
                    .validator(is_int)
                ])
        )
}

pub fn arg_value_as_str(matches: Option<&ArgMatches>, arg: &str) -> String {
    match matches {
        Some(x) => match x.value_of(arg) {
                        Some(y) => y.to_string(),
                        None => "".to_string()
        },
        None => "".to_string()
    }
}

pub fn arg_value_as_u16(matches: Option<&ArgMatches>, arg: &str) -> u16 {
    match matches {
        Some(x) => match x.value_of(arg) {
                        Some(y) => y.parse::<u16>().unwrap(),
                        None => 0
        },
        None => 0
    }
}