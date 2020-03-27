use clap::{Arg, App, ArgMatches, SubCommand};
use log::{error};

pub struct Args {
    pub command: String,
    pub config_file: String,
    pub collection_file: String,
    pub delay: u64,
    pub execution_time: u64,
    pub handle_cookies: bool,
    pub iterations: u64,
    pub ramp_up: u64,
    pub report: String,
    pub threads: u64,
    pub continue_on_failure: bool,
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
    let arg_continue_on_failure = "continue on failure";
    
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
        .subcommand(SubCommand::with_name("bombard").about("Executes the test")
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
                        .display_order(7)
                        .help("handle cookies"))
                .arg(Arg::with_name(arg_continue_on_failure)
                        .short("o")
                        .display_order(8)
                        .help("continue on failure")))
        .subcommand(SubCommand::with_name("report")
                .about("Generates the report from report file")
                .arg(Arg::with_name(arg_report)
                        .short("r")
                        .takes_value(true)
                        .default_value("report.csv")
                        .display_order(6)
                        .help("report file path")))
        .get_matches();

    
    let (subcommand, subcommand_args) = matches.subcommand();


    let args = Args {
        command: subcommand.to_string(),
        collection_file: get_value_as_str(subcommand_args, arg_collection),
        config_file: get_value_as_str(subcommand_args, arg_config),
        delay: get_value_as_u64(subcommand_args, arg_delay),
        execution_time: get_value_as_u64(subcommand_args, arg_execution_time),
        handle_cookies: get_value_as_bool(subcommand_args, arg_cookies),
        iterations: get_value_as_u64(subcommand_args, arg_iterations),
        ramp_up: get_value_as_u64(subcommand_args, arg_ramp_up),
        report: get_value_as_str(subcommand_args, arg_report),
        threads: get_value_as_u64(subcommand_args, arg_threads),
        continue_on_failure: get_value_as_bool(subcommand_args,arg_continue_on_failure),
    };

    //More validations on args
    if args.execution_time > 0 && args.execution_time < args.ramp_up {
        error!("Ramp up time should be less than Execution time");
        return Err(());
    }

    Ok(args)
}

fn get_value_as_bool(matches: Option<&ArgMatches>, arg: &str) -> bool {
    match matches {
        Some(x) => x.is_present(arg),
        None => false
    }
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

fn get_value_as_u64(matches: Option<&ArgMatches>, arg: &str) -> u64 {
    match matches {
        Some(x) => match x.value_of(arg) {
                        Some(y) => {
                            let arg: String = String::from(y);
                            arg.parse::<u64>().unwrap()
                        },
                        None => 0
        },
        None => 0
    }
}