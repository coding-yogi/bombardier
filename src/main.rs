mod bombardier;
mod cmd;
mod model;
mod parse;
mod protocol;
mod report;
mod server;
mod storage;
mod util;

use log::{info, error};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    bombardier::Bombardier, 
    parse::parser, 
    protocol::socket, 
    report::stats, 
    util::{logger, file}
};

#[tokio::main]
async fn main()  {

    logger::initiate(true);

    let matches = cmd::create_cmd_app().get_matches();
    let (subcommand, subcommand_args) = matches.subcommand();

    if subcommand == "" {
        error!("No subcommand found. Should either be 'bombard', 'report' or 'node'");
        return;
    }

    match subcommand {
        "bombard" => {
            let config_file_path = cmd::arg_value_as_str(subcommand_args, cmd::CONFIG_FILE_ARG_NAME);
            info!("Parsing config file {}", config_file_path);

            let config_content = match file::get_content(&config_file_path).await {
                Err(err) => {
                    error!("Error while reading config file {}", err);
                    return
                },
                Ok(content) => content
            };

            let config = parser::parse_config_from_string(config_content).unwrap();

            //get content of env file
            let env_file_path = cmd::arg_value_as_str(subcommand_args, cmd::ENVIRONMENT_FILE_ARG_NAME);
            let mut env_content = String::new();
            if env_file_path != "" {
                info!("Reading environments file {}", env_file_path);
                env_content = match file::get_content(&env_file_path).await {
                    Ok(content) => content,
                    Err(err) => {
                        error!("Error while reading env file {}", err);
                        return
                    }
                };
            }

            //get content of scenario file
            let scenarios_file = cmd::arg_value_as_str(subcommand_args, cmd::SCENARIOS_FILE_ARG_NAME);
            info!("Reading scenarios file {}", &scenarios_file);
            let scenarios_content = match file::get_content(&scenarios_file).await {
                Err(err) => {
                    error!("Error while reading scenarios file {}", err);
                    return
                },
                Ok(content) => content
            };

            //get data file content
            let data_file = cmd::arg_value_as_str(subcommand_args, cmd::DATA_FILE_ARG_NAME);
            let mut data_content = String::new();
            if data_file != "" {
                info!("Reading data file {}", data_file);
                data_content = match file::get_content(&data_file).await {
                    Ok(content) => content,
                    Err(err) => {
                        error!("Error while data file {}", err);
                        return
                    }
                };
            }

            info!("Prepare bombardier");
            let bombardier = 
                    Bombardier::new(config, env_content, scenarios_content, data_content).await.unwrap();
            
            let (stats_sender,  stats_receiver_handle) = 
            match stats::StatsConsumer::new(&bombardier.config, Arc::new(Mutex::new(None))).await {
                Ok((s,r)) => (s,r),
                Err(err) => {
                    error!("Error while initializing stats consumer {}", err);
                    return
                }
            };

            info!("Bombarding !!!");
            match bombardier.bombard(stats_sender).await {
                Err(err) => error!("Bombarding failed : {}", err),
                Ok(()) => info!("Bombarding Complete. Run report command to get details")
            }   

            stats_receiver_handle.await.unwrap();
        },
        "report" => {
            let report_file = cmd::arg_value_as_str(subcommand_args, cmd::REPORT_FILE_ARG_NAME);

            info!("Generating report");
            match report::display(&report_file).await {
                Err(err) => {
                    error!("Error while displaying reports : {}", err);
                    return;
                },
                Ok(()) => ()
            }
        },
        "node" => {
            let hub_address = cmd::arg_value_as_str(subcommand_args, cmd::HUB_ADDRESS_ARG_NAME);

            info!("Starting bombardier as a node");
            match  server::node::start(hub_address).await {
                Err(err) => {
                    error!("Error occured in the node : {}", err);
                    return;
                },
                Ok(()) => ()
            }; 
        },
        "hub" => {
            let server_port = cmd::arg_value_as_u16(subcommand_args, cmd::SERVER_PORT_ARG_NAME);
            let ws_port = cmd::arg_value_as_u16(subcommand_args, cmd::SOCKET_PORT_ARG_NAME);

            info!("Starting bombardier as a hub server");
            match server::servers::serve(server_port, ws_port).await {
                Err(err) => {
                    error!("Error occured while running bombardier as server : {}", err);
                    return;
                },
                Ok(()) => ()
            }
        },
        _ => {
            error!("Invalid command");
        },
    }
}