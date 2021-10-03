pub mod cmd;
pub mod logger;

mod bombardier;
mod converter;
mod data;
mod model;
mod parse;
mod protocol;
mod report;
mod server;
mod storage;

use cmd::App;
use log::{info, error};
use std::sync::Arc;
use tokio::{fs,sync::Mutex};

use crate::{
    bombardier::Bombardier, 
    model::Config,
    parse::parser, 
    report::stats
};

pub async fn process_subcommand(app: App<'_>) {

    let subcommand = app.subcommand();
    if subcommand.is_empty() {
        error!("No subcommand found. Should either be 'bombard', 'report', 'hub' or 'node'");
        return;
    }

    match subcommand.as_str() {
        "bombard" => bombard(app).await,
        "report" => report(app).await,
        "node" => node(app).await,
        "hub" => hub(app).await,
        _ => error!("Invalid command")
    }
}

async fn bombard(app: App<'_>) {
    //Get config
    let config_file_path = app.arg_value_as_str(cmd::CONFIG_FILE_ARG_NAME);
    let mut config = match get_config(&config_file_path).await {
        Some(c) => c,
        None => return
    };

    //Get content of env file
    let env_file_path = app.arg_value_as_str(cmd::ENVIRONMENT_FILE_ARG_NAME);
    let env_content = match get_file_content(&env_file_path).await {
        Some(c) => c,
        None => return
    };

    //Get content of env file
    let scenarios_file_path = app.arg_value_as_str(cmd::SCENARIOS_FILE_ARG_NAME);
    let scenarios_content = match get_file_content(&scenarios_file_path).await {
        Some(c) => c,
        None => return
    };

    //Get data file path
    config.data_file = app.arg_value_as_str(cmd::DATA_FILE_ARG_NAME);

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
}

async fn report(app: App<'_>) {
    let report_file = app.arg_value_as_str(cmd::REPORT_FILE_ARG_NAME);

    info!("Generating report");
    if let Err(err) =  report::display(&report_file).await {
        error!("Error while displaying reports : {}", err)
    }
}

async fn node(app: App<'_>) {
    let hub_address = app.arg_value_as_str(cmd::HUB_ADDRESS_ARG_NAME);

    info!("Starting bombardier as a node");
    if let Err(err) =  server::node::start(hub_address).await {
        error!("Error occured in the node : {}", err)
    }
}

async fn hub(app: App<'_>) {
    let server_port = app.arg_value_as_u16(cmd::SERVER_PORT_ARG_NAME);
    let ws_port = app.arg_value_as_u16(cmd::SOCKET_PORT_ARG_NAME);

    info!("Starting bombardier as a hub server");
    if let Err(err) = server::servers::serve(server_port, ws_port).await {
        error!("Error occured while running bombardier as server : {}", err)
    }
}

async fn get_config(file_path: &str) -> Option<Config> { 
    info!("Parsing config file {}", file_path);
    if let Some(config_content) = get_file_content(file_path).await {
        if let Ok(config) = parser::parse_config(config_content) {
            return Some(config);
        }
    }

    None
}

async fn get_file_content(file_path: &str) -> Option<String> {
    if !file_path.is_empty() {
        info!("Reading {} file", file_path);
        return match fs::read_to_string(file_path).await {
            Ok(content) => Some(content),
            Err(err) => {
                error!("Error while reading file {}", err);
                None
            }
        }
    }

    None
}