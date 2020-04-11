mod parser;
mod file;
mod cmd;
mod bombardier;
mod http;
mod report;
mod influxdb;
mod postprocessor;

use log::{info, error};

fn main()  {
    pretty_env_logger::init_timed();

    let args = cmd::get_args()
        .expect("Args validation failed");

    match args.command.as_str() {
        "bombard" => {
            info!("Reading collections file");
            let contents = match file::get_content(&args.collection_file) {
                Err(err) => {
                    error!("Error occured while reading collection file {} : {}", &args.collection_file, err);
                    return;
                },
                Ok(c) => c
            };
            
            info!("Reading environments file");
            let env_map = match parser::get_env_map(&args.environment_file) {
                Err(err) => {
                    error!("Error occured while reading environments file {} : {}", &args.environment_file, err);
                    return;
                },
                Ok(map) => map
            }; 

            info!("Reading data file");
            let vec_data_map = match parser::get_vec_data_map(&args.data_file) {
                Err(err) => {
                    error!("Error occured while reading data file {} : {}", &args.data_file, err);
                    return;
                },
                Ok(vec) => vec
            };

            info!("Generating bombardier requests");
            let requests = match parser::parse_requests(contents, &env_map) {
                Err(err) => {
                    error!("Error occured while parsing requests : {}", err);
                    return;
                },
                Ok(v) => v
            };
           
            info!("Bombarding !!!");
            match bombardier::bombard(args, env_map, requests, vec_data_map) {
                Err(err) => error!("Bombarding failed : {}", err),
                Ok(()) => ()
            }   

            info!("Execution Complete. Run report command to get details");
        },
        "report" => {
            info!("Generating report");
            match report::display(args.report_file) {
                Err(err) => {
                    error!("Error occured while generating report : {}", err);
                    return;
                },
                Ok(()) => ()
            }; 
        },
        _ => {
            error!("Invalid command");
        }
    }
}
