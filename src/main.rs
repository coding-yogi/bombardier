mod bombardier;
mod cmd;
mod model;
mod parse;
mod protocol;
mod report;
mod server;
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
            let config_file_path = cmd::get_config_file_path(subcommand_args);
            info!("Parsing config file {}", config_file_path);

            let config_content = match file::get_content(&config_file_path).await {
                Err(_) => return,
                Ok(content) => content
            };

            let config = parser::parse_config_from_string(config_content).unwrap();

            //get content of env file
            let mut env_content = String::new();
            if config.environment_file != "" {
                info!("Reading environments file {}", config.environment_file);
                env_content = match file::get_content(&config.environment_file).await {
                    Ok(content) => content,
                    Err(_) => return
                };
            }

            //get content of scenario file
            info!("Reading scenarios file {}", &config.scenarios_file);
            let scenarios_content = match file::get_content(&config.scenarios_file).await {
                Err(_) => return,
                Ok(content) => content
            };

            //get data file content
            let mut data_content = String::new();
            let data_file = &config.data_file;
            if data_file != "" {
                info!("Reading data file {}", data_file);
                data_content = match file::get_content(&data_file).await {
                    Ok(content) => content,
                    Err(_) => return
                };
            }

            info!("Prepare bombardier");
            let bombardier = 
                    Bombardier::new(config, env_content, scenarios_content, data_content).await.unwrap();
            
            let (stats_sender,  stats_receiver_handle) = 
            stats::StatsConsumer::new(&bombardier.config, Arc::new(Mutex::new(None))).await;

            info!("Bombarding !!!");
            match bombardier.bombard(stats_sender).await {
                Err(err) => error!("Bombarding failed : {}", err),
                Ok(()) => info!("Bombarding Complete. Run report command to get details")
            }   

            stats_receiver_handle.await.unwrap();
            
        },
        "report" => {
            let report_file = cmd::get_report_file(subcommand_args);

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
            info!("Starting bombardier as a node");
            let hub_address = cmd::get_hub_address(subcommand_args);

            match  server::node::start(hub_address).await {
                Err(err) => {
                    error!("Error occured in the node : {}", err);
                    return;
                },
                Ok(()) => ()
            }; 
        },
        "hub" => {
            info!("Starting bombardier as a hub server");
            let server_port = cmd::get_port(subcommand_args, cmd::SERVER_PORT_ARG_NAME);
            let ws_port = cmd::get_port(subcommand_args, cmd::SOCKET_PORT_ARG_NAME);
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