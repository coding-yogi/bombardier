mod bombardier;
mod cmd;
mod parse;
mod protocol;
mod report;
mod scale;
mod util;

use log::{info, error};
use std::sync::{Arc, Mutex};
use crate::{
    parse::{model, parser},
    protocol::socket,
    report::stats,
    scale::{hub, node},
    util::{logger, file},
};

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
                    error!("Error occured while parsing config json : {}", err);
                    return;
                },
                Ok(config) => config
            }; 

            let scenarios_file = &config.scenarios_file;
            info!("Reading scenarios file {}", scenarios_file);
            let contents = match file::get_content(scenarios_file) {
                Err(err) => {
                    error!("Error occured while reading scenarios file {} : {}", scenarios_file, err);
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
                match hub::distribute(config, env_map, requests) {
                    Err(err) => error!("Load distribution failed : {}", err),
                    Ok(()) => ()
                }
            } else {
                info!("Bombarding !!!");
                let bombardier = bombardier::Bombardier {
                    config,
                    env_map,
                    requests,
                    vec_data_map
                };

                let websocket = Arc::new(Mutex::new(None));
                let (stats_sender,  stats_receiver_handle) = stats::StatsConsumer::new(&bombardier.config, websocket);

                match bombardier.bombard(stats_sender) {
                    Err(err) => error!("Bombarding failed : {}", err),
                    Ok(()) => info!("Bombarding Complete. Run report command to get details")
                }   

                stats_receiver_handle.join().unwrap();
            }
        },
        "report" => {
            let report_file = cmd::get_report_file(subcommand_args);

            info!("Generating report");
            match stats::display(&report_file) {
                Err(err) => {
                    error!("Error while displaying reports : {}", err);
                    return;
                },
                Ok(()) => ()
            }
        },
        "node" => {
            info!("Starting bombardier as a node");
            let port = cmd::get_port(subcommand_args);
            match  node::serve(&port.to_string()) {
                Err(err) => {
                    error!("Error occured while running bombardier as node : {}", err);
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
