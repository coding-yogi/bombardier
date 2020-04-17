mod parser;
mod file;
mod cmd;
mod bombardier;
mod http;
mod report;
mod influxdb;
mod postprocessor;

use log::{info, error};
use figlet_rs::FIGfont;

fn main()  {
    pretty_env_logger::init_timed();

    let config = match cmd::get_config() {
        Err(err) => {
            error!("Error occured while parsing command line args : {}", err);
            return;
        },
        Ok(config) => config
    }; 

    match config.command.as_str() {
        "bombard" => {

            let standard_font = FIGfont::standand().unwrap();
            let figure = standard_font.convert("Bombardier");
            println!("{}", figure.unwrap());
            
            info!("Reading collection file");
            let contents = match file::get_content(&config.collection_file) {
                Err(err) => {
                    error!("Error occured while reading collection file {} : {}", &config.collection_file, err);
                    return;
                },
                Ok(c) => c
            };
            
            info!("Reading environments file");
            let env_map = match parser::get_env_map(&config.environment_file) {
                Err(err) => {
                    error!("Error occured while reading environments file {} : {}", &config.environment_file, err);
                    return;
                },
                Ok(map) => map
            }; 

            info!("Reading data file");
            let vec_data_map = match parser::get_vec_data_map(&config.data_file) {
                Err(err) => {
                    error!("Error occured while reading data file {} : {}", &config.data_file, err);
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
            match bombardier::bombard(config, env_map, requests, vec_data_map) {
                Err(err) => error!("Bombarding failed : {}", err),
                Ok(()) => ()
            }   

            info!("Execution Complete. Run report command to get details");
        },
        "report" => {
            info!("Generating report");
            match report::display(config.report_file) {
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
