mod bombardier;
mod cmd;
mod distributor;
mod file;
mod http;
mod influxdb;
mod logger;
mod node;
mod parser;
mod postprocessor;
mod report;
mod socket;

use log::{info, error};

fn main()  {

    logger::initiate(true);

    let matches = cmd::create_cmd_app().get_matches();
    let (subcommand, subcommand_args) = matches.subcommand();

    if subcommand == "" {
        error!("No subcommand found. Should either be 'bombard', 'report' or 'node'");
        return;
    }

    match subcommand {
        "bombard" => {
            let config = match cmd::get_config(subcommand_args) {
                Err(err) => {
                    error!("Error occured while parsing command line args : {}", err);
                    return;
                },
                Ok(config) => config
            }; 

            let collection_file = &config.collection_file;
            info!("Reading collections file {}", collection_file);
            let contents = match file::get_content(collection_file) {
                Err(err) => {
                    error!("Error occured while reading collection file {} : {}", collection_file, err);
                    return;
                },
                Ok(c) => c
            };
            
            let env_file = &config.environment_file;
            info!("Reading environments file {}", env_file);
            let env_map = match parser::get_env_map(env_file) {
                Err(err) => {
                    error!("Error occured while reading environments file {} : {}", env_file, err);
                    return;
                },
                Ok(map) => map
            }; 

            let data_file = &config.data_file;
            info!("Reading data file {}", data_file);
            let vec_data_map = match parser::get_vec_data_map(data_file) {
                Err(err) => {
                    error!("Error occured while reading data file {} : {}", data_file, err);
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
           
            if config.distributed {
                info!("Distributed bombarding is set to true");
                match distributor::distribute(config, env_map, requests) {
                    Err(err) => error!("Load distribution failed : {}", err),
                    Ok(()) => ()
                }
            } else {
                info!("Bombarding !!!");
                let bombardier = bombardier::Bombardier {
                    config,
                    env_map,
                    requests,
                    vec_data_map,
                    ws: None
                };

                match bombardier.bombard() {
                    Err(err) => error!("Bombarding failed : {}", err),
                    Ok(()) => ()
                }   

                info!("Execution Complete. Run report command to get details");
            }
            
        },
        "report" => {
            let config = match cmd::get_config(subcommand_args) {
                Err(err) => {
                    error!("Error occured while parsing command line args : {}", err);
                    return;
                },
                Ok(config) => config
            }; 

            info!("Generating report");
            match report::display(config.report_file) {
                Err(err) => {
                    error!("Error occured while generating report : {}", err);
                    return;
                },
                Ok(()) => ()
            }; 
        },
        "node" => {
            info!("Starting bombardier as a node");
            let port = cmd::get_port(subcommand_args);
            node::serve(&port.to_string())
        },
        _ => {
            error!("Invalid command");
        }
    }
}
