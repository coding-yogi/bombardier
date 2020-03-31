mod parser;
mod file;
mod cmd;
mod executor;
mod http;
mod report;
mod influxdb;

use log::{info, error};

fn main() {
    pretty_env_logger::init_timed();

    let args = cmd::get_args()
        .expect("Args validation failed");

    match args.command.as_str() {
        "bombard" => {
            // Get scenarios
            info!("Reading collections file");
            let mut contents = file::get_content(&args.collection_file);
            
            //Get config
            info!("Reading environments file");
            let config_content = file::get_content(&args.environment_file);
            let env_map = parser::get_env(&config_content);

            info!("Generating bombardier requests");
            contents = file::find_and_replace(contents, &env_map);
            let requests = parser::parse_requests(&contents);
           
            info!("Bombarding !!!");
            executor::execute(args, env_map, requests);

            info!("Execution Complete. Run report command to get details");
        },
        "report" => {
            info!("Generating report");
            report::display(args.report_file); 
        },
        _ => {
            error!("Invalid command");
        }
    }
}