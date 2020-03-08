use log::{info, debug};

mod parser;
mod file;
mod cmd;

fn main() {
    env_logger::init();

    let args = cmd::get_args()
        .expect("Args validation failed");

    info!("Executing Tests ...");

     // Get scenarios
    info!("Reading collections file");
    let mut contents = file::get_content("scenarios/collection.json");
    
    //Get config
    info!("Reading environments file");
    let config_content = file::get_content("scenarios/environment.json");
    let env_map = parser::get_env(&config_content);

    info!("Replacing parameter values");
    contents = file::find_and_replace(contents, env_map);
    
    info!("Get scenarios");
    let root: parser::Root = parser::get_scenarios(&contents)
        .expect("Failed while pasring JSON");

    //DEBUG: Print scenarios
    debug!("Logging requests");
    for scenario in &root.scenarios {
        println!("{:?}", scenario.request_details);
        for request in &scenario.requests {
            debug!("{:?}", request.request_details)
        }
    }
     
}


