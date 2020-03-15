mod parser;
mod file;
mod cmd;
mod executor;
mod http;
mod report;

use std::time;
use log::{info};

fn main() {
    pretty_env_logger::init_timed();

    let args = cmd::get_args()
        .expect("Args validation failed");

     // Get scenarios
    info!("Reading collections file");
    let mut contents = file::get_content(&args.collection_file);
    
    //Get config
    info!("Reading environments file");
    let config_content = file::get_content(&args.config_file);
    let env_map = parser::get_env(&config_content);

    //Replacing parameter values
    contents = file::find_and_replace(contents, env_map);
    
    info!("Generating bombardier requests");
    let requests = parser::parse_requests(&contents);
    let names = parser::get_request_names(&requests);

    info!("Bombarding !!!");
    let st = time::Instant::now();
    let stats = executor::execute(args, requests);
    let et = st.elapsed().as_secs();

    info!("Generating report");
    report::generate_report(names, stats, et); 
}