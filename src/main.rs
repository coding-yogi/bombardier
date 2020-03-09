mod parser;
mod file;
mod cmd;
mod massager;
mod executor;

use log::{info, debug};

fn main() {
    env_logger::init();

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
    
    info!("Parsing scenarios");
    let mut scenarios = parser::parse_scenarios(&contents);
        
    info!("Massaging requests");
    massager::massage(&mut scenarios);

    info!("Executing tests");
    executor::execute(&args, &scenarios);
}