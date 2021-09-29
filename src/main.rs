mod bombardier;
mod cmd;
mod data;
mod logger;
mod model;
mod parse;
mod protocol;
mod report;
mod server;
mod storage;

use cmd::ExecConfig;
use log::{info, error};
use std::sync::Arc;
use tokio::{fs,sync::Mutex};

use crate::{
    bombardier::Bombardier, 
    parse::parser, 
    protocol::socket, 
    report::stats
};

#[tokio::main]
async fn main()  {

    logger::initiate(true);

    let matches = cmd::create_cmd_app().get_matches();
    let (subcommand, arg_matches) = matches.subcommand();

    if subcommand == "" {
        error!("No subcommand found. Should either be 'bombard', 'report' or 'node'");
        return;
    }

    match subcommand {
        "bombard" => {
            //Get config
            let mut config = match get_config(arg_matches).await {
                Some(c) => c,
                None => return
            };

            //Get content of env file
            let env_content = match get_arg_file_content(arg_matches, cmd::ENVIRONMENT_FILE_ARG_NAME).await {
                Some(c) => c,
                None => return
            };

            //Get content of env file
            let scenarios_content = match get_arg_file_content(arg_matches, cmd::SCENARIOS_FILE_ARG_NAME).await {
                Some(c) => c,
                None => return
            };

            //Get data file path
            config.data_file = cmd::arg_value_as_str(arg_matches, cmd::DATA_FILE_ARG_NAME);

            info!("Prepare bombardier");
            let bombardier = Bombardier::new(config, env_content, scenarios_content).unwrap();
            
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
            let report_file = cmd::arg_value_as_str(arg_matches, cmd::REPORT_FILE_ARG_NAME);

            info!("Generating report");
            match report::display(&report_file).await {
                Err(err) => error!("Error while displaying reports : {}", err),
                Ok(()) => ()
            }
        },
        "node" => {
            let hub_address = cmd::arg_value_as_str(arg_matches, cmd::HUB_ADDRESS_ARG_NAME);

            info!("Starting bombardier as a node");
            match  server::node::start(hub_address).await {
                Err(err) => error!("Error occured in the node : {}", err),
                Ok(()) => ()
            }; 
        },
        "hub" => {
            let server_port = cmd::arg_value_as_u16(arg_matches, cmd::SERVER_PORT_ARG_NAME);
            let ws_port = cmd::arg_value_as_u16(arg_matches, cmd::SOCKET_PORT_ARG_NAME);

            info!("Starting bombardier as a hub server");
            match server::servers::serve(server_port, ws_port).await {
                Err(err) => error!("Error occured while running bombardier as server : {}", err),
                Ok(()) => ()
            }
        },
        _ => {
            error!("Invalid command");
        },
    }
}

async fn get_config<'a>(args_match: Option<&clap::ArgMatches<'a>>) -> Option<ExecConfig> {
    let config_file_path = cmd::arg_value_as_str(args_match, cmd::CONFIG_FILE_ARG_NAME);
    info!("Parsing config file {}", config_file_path);

    let config_content = match fs::read_to_string(&config_file_path).await {
        Err(err) => {
            error!("Error while reading config file {}", err);
            return None;
        },
        Ok(content) => content
    };

    match parser::parse_config(config_content) {
        Ok(c) => Some(c),
        Err(_) => None
    }
}

async fn get_arg_file_content<'a>(args_match: Option<&clap::ArgMatches<'a>>, arg_name: &str) -> Option<String> {
    let file_path = cmd::arg_value_as_str(args_match, arg_name);
    let mut content = String::new();
    if file_path != "" {
        info!("Reading {} file", file_path);
        content = match fs::read_to_string(&file_path).await {
            Ok(content) => content,
            Err(err) => {
                error!("Error while reading file {}", err);
                return None
            }
        };
    } else {
        info!("No file provided for {}", arg_name);
    }

    Some(content)
}