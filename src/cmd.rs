use crate::file;

use std::process;

use clap::{Arg, App, ArgMatches, SubCommand};
use serde::{Serialize, Deserialize, Deserializer, de::Error};
use log::{error, info, warn};


#[derive(Serialize, Deserialize, Debug)]
pub struct Args {

    #[serde(default)]
    pub command: String,

    #[serde(deserialize_with = "check_json_file")]
    pub environment_file: String,

    #[serde(deserialize_with = "check_json_file")]
    pub collection_file: String,

    pub report_file: String,

    #[serde(deserialize_with = "check_non_zero")]
    pub thread_count: u64,

    #[serde(default)]
    pub iterations: u64,

    #[serde(default)]
    pub execution_time: u64,

    #[serde(default)]
    pub thread_delay: u64,

    #[serde(deserialize_with = "check_non_zero")]
    pub rampup_time: u64,
    
    #[serde(default)]
    pub handle_cookies: bool,

    #[serde(default)]
    pub continue_on_error: bool,

    #[serde(default)]
    pub influxdb: InfluxDB
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct InfluxDB {
    pub host: String,
    pub port: u64,
    pub dbname: String,
}


pub fn get_args() -> Result<Args, Box<dyn std::error::Error + Send + Sync>> {

    let config_arg_name = "config json file";
    let config_arg = Arg::with_name(config_arg_name)
                        .short("c")
                        .long("config")
                        .takes_value(true)
                        .required(true)
                        .validator(|s: String| {
                            match s.ends_with(".json") {
                                true => Ok(()),
                                false => Err(String::from("File should be a .json file"))
                            }
                        })
                        .display_order(0)
                        .help("Execution configuration json file");

    let matches = App::new("Bombardier")
        .version("0.1.0")
        .author("Coding Yogi <aniket.g2185@gmail.com>")
        .subcommand(SubCommand::with_name("bombard")
                .about("Executes the test")
                .arg(&config_arg))
        .subcommand(SubCommand::with_name("report")
                .about("Generates the report from report file")
                .arg(&config_arg))
        .get_matches();

    let (subcommand, subcommand_args) = matches.subcommand();
    if subcommand == "" {
        error!("No command found. Command should either be 'bombard' or 'report'");
        process::exit(-1);
    }

    let config_file_path = get_value_as_str(subcommand_args, config_arg_name);
    info!("Parsing config file {}", config_file_path);
    
    let content = file::get_content(&config_file_path);
    let mut args: Args = match serde_json::from_str(&content) {
        Ok(a) => a,
        Err(err) => {
            error!("Error while deserializing config json: {}", err);
            process::exit(-1);
        }
    };
    args.command = subcommand.to_string();

    if args.execution_time == 0 && args.iterations == 0 {
        error!("Both execution time and iterations cannot be 0");
        process::exit(-1);
    }

    if args.execution_time > 0 && args.iterations > 0 {
        warn!("Both execution time and iterations values provided. Execution time will be ignored");
    }

    Ok(args)
}

fn check_non_zero <'de, D>(deserializer: D) -> Result<u64, D::Error> 
where D: Deserializer<'de> {
    
    let val = u64::deserialize(deserializer)?;
    if val == 0 {
        return Err(Error::custom("Value cannot be zero"))
    }

    Ok(val)
}

fn check_json_file <'de, D>(deserializer: D) -> Result<String, D::Error> 
where D: Deserializer<'de> {
    
    let val = String::deserialize(deserializer)?;
    if !val.ends_with(".json")  {
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