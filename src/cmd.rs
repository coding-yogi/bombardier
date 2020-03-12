use log::{error};
use clap::{Arg, App, ArgMatches};

pub struct Args {
    pub config_file: String,
    pub collection_file: String,
    pub threads: u64,
    pub ramp_up: u64,
    pub execution_time: u64
}

pub fn get_args() -> Result<Args, ()> {
    let arg_config = "config";
    let arg_collection = "collection";
    let arg_threads = "threads";
    let arg_ramp_up = "rampup";
    let arg_execution_time = "execution_time";

    let is_json_file = |s: String| {
        match s.ends_with(".json") {
            true => Ok(()),
            false => Err(String::from("File should be a .json file"))
        }
    };

    let is_number = |s: String| {
        match s.parse::<u64>().is_err() {
            true => Err(String::from("Argument should be a number")),
            false =>  match s.parse::<u64>().unwrap() {
                0 => Err(String::from("Argument cannot be 0")),
                _ => Ok(())
            }
        }
    };

    let matches = App::new("Bombardier")
        .version("0.1.0")
        .author("Coding Yogi <aniket.g2185@gmail.com>")
        .arg(Arg::with_name(arg_config)
                 .short("c")
                 .takes_value(true)
                 .required(true)
                 .validator(is_json_file)
                 .help("Environments JSON file"))
        .arg(Arg::with_name(arg_collection)
                 .short("f")
                 .takes_value(true)
                 .required(true)
                 .validator(is_json_file)
                 .help("Collections JSON file"))
        .arg(Arg::with_name(arg_threads)
                 .short("t")
                 .takes_value(true)
                 .required(true)
                 .validator(is_number)
                 .help("Load in number of threads"))
        .arg(Arg::with_name(arg_ramp_up)
                 .short("r")
                 .takes_value(true)
                 .validator(is_number)
                 .required(true)
                 .help("Ramp up time for given users in secs"))
        .arg(Arg::with_name(arg_execution_time)
                 .short("e")
                 .takes_value(true)
                 .required(true)
                 .validator(is_number)
                 .help("Execution time in secs"))
        .get_matches();


    let args = Args {
        collection_file: get_value_as_str(&matches, arg_collection),
        config_file: get_value_as_str(&matches, arg_config),
        threads: get_value_as_u64(&matches, arg_threads),
        ramp_up: get_value_as_u64(&matches, arg_ramp_up),
        execution_time: get_value_as_u64(&matches, arg_execution_time)
    };

    //More validations on args
    if args.execution_time < args.ramp_up {
        error!("Ramp up time should be less than Execution time");
        return Err(());
    }

    Ok(args)
}

fn get_value_as_str(matches: &ArgMatches, arg: &str) -> String {
    matches.value_of(arg).unwrap().to_string()
}

fn get_value_as_u64(matches: &ArgMatches, arg: &str) -> u64 {
    let arg: String = String::from(matches.value_of(arg).unwrap());
    let uarg: u64 = arg.parse::<u64>().unwrap();
    uarg
}