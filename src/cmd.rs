use clap::{Arg, App, ArgMatches};
use log::{error};

pub struct Args {
    pub config_file: String,
    pub collection_file: String,
    pub delay: u64,
    pub execution_time: u64,
    pub handle_cookies: bool,
    pub iterations: u64,
    pub ramp_up: u64,
    pub report: String,
    pub threads: u64,
}

pub fn get_args() -> Result<Args, ()> {
    let arg_config = "config";
    let arg_collection = "collection";
    let arg_delay = "delay";
    let arg_execution_time = "execution time";
    let arg_cookies = "handle cookies";
    let arg_iterations = "iterations";  
    let arg_ramp_up = "rampup";
    let arg_report = "report";
    let arg_threads = "threads";
    
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
                 .display_order(0)
                 .help("Environments JSON file"))
        .arg(Arg::with_name(arg_collection)
                 .short("f")
                 .takes_value(true)
                 .required(true)
                 .validator(is_json_file)
                 .display_order(1)
                 .help("Collections JSON file"))
        .arg(Arg::with_name(arg_threads)
                 .short("t")
                 .takes_value(true)
                 .required(true)
                 .display_order(2)
                 .validator(is_number)
                 .help("Load in number of threads"))
        .arg(Arg::with_name(arg_ramp_up)
                 .short("u")
                 .takes_value(true)
                 .validator(is_number)
                 .display_order(3)
                 .required(true)
                 .help("Ramp up time for given users in secs"))
        .arg(Arg::with_name(arg_execution_time)
                 .short("e")
                 .takes_value(true)
                 .conflicts_with(arg_iterations)
                 .validator(is_number)
                 .display_order(4)
                 .help("Execution time in secs"))
        .arg(Arg::with_name(arg_iterations)
                 .short("i")
                 .takes_value(true)
                 .required_unless(arg_execution_time)
                 .validator(is_number)
                 .display_order(4)
                 .help("Iterations"))
        .arg(Arg::with_name(arg_delay)
                 .short("d")
                 .takes_value(true)
                 .default_value("1")
                 .validator(is_number)
                 .display_order(5)
                 .help("Delay between requests in ms"))
        .arg(Arg::with_name(arg_report)
                 .short("r")
                 .takes_value(true)
                 .default_value("report.csv")
                 .display_order(6)
                 .help("report file path"))
        .arg(Arg::with_name(arg_cookies)
                 .short("h")
                 .help("handle cookies"))
                 .display_order(7)
        .get_matches();


    let args = Args {
        collection_file: get_value_as_str(&matches, arg_collection),
        config_file: get_value_as_str(&matches, arg_config),
        delay: get_value_as_u64(&matches, arg_delay),
        execution_time: get_value_as_u64(&matches, arg_execution_time),
        handle_cookies: matches.is_present(arg_cookies),
        iterations: get_value_as_u64(&matches, arg_iterations),
        ramp_up: get_value_as_u64(&matches, arg_ramp_up),
        report: get_value_as_str(&matches, arg_report),
        threads: get_value_as_u64(&matches, arg_threads),
    };

    //More validations on args
    if args.execution_time > 0 && args.execution_time < args.ramp_up {
        error!("Ramp up time should be less than Execution time");
        return Err(());
    }

    Ok(args)
}

fn get_value_as_str(matches: &ArgMatches, arg: &str) -> String {
    match matches.value_of(arg) {
        Some(x) => x.to_string(),
        None => "".to_string()
    }
}

fn get_value_as_u64(matches: &ArgMatches, arg: &str) -> u64 {
    match matches.value_of(arg) {
        Some(x) => {
            let arg: String = String::from(x);
            let uarg: u64 = arg.parse::<u64>().unwrap();
            uarg
        },

        None => 0
    } 
}