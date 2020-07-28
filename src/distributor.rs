use crate::cmd;
use crate::parser;
use crate::report;
use crate::socket;

use std::collections::HashMap;
use std::fs::File;
use std::thread;

use log::{debug, info};
use ws::{Builder, Handler, Message, Result as WsResult};


struct Distributor {
    report_file: File
}

impl Handler for Distributor {

    fn on_message(&mut self, msg: Message) -> WsResult<()> {
        let raw_message = msg.as_text()?;
        debug!("Message received from the node {:#?}", &raw_message);

        //This should be report stat written to CSV file (if possible asynchronously)
        let stats: report::Stats = serde_json::from_str(raw_message).unwrap();
        report::write_stats_to_csv(&mut self.report_file, &format!("{}", &stats));
        
        Ok(())
    }
}

pub fn distribute(config: cmd::ExecConfig, env_map: HashMap<String, String>, requests: Vec<parser::Request>) -> Result<(), Box<dyn std::error::Error>> {

    let no_of_nodes = config.nodes.len();
    if no_of_nodes == 0 {
        return Err(Box::from("No nodes mentioned for distributed bombarding"));
    }

    //let mut clients = vec![];
    info!("Creating report file: {}", &config.report_file);
    let report_file = report::create_file(&config.report_file).unwrap();

    let mut websocket = Builder::new()
        .build(move |_| Distributor {
            report_file: report_file.try_clone().unwrap()
        })?;

    let message = socket::Message {
        config: config.clone(),
        env_map: env_map,
        requests: requests
    };
    
    for node in config.nodes {
        let url = url::Url::parse(&format!("ws://{}/ws", &node)).unwrap();
        websocket.connect(url).unwrap();
    }

    let broadcaster = websocket.broadcaster();
    let handle = thread::spawn(move || {
        websocket.run().unwrap();
    });

    broadcaster.send(serde_json::to_string(&message).unwrap())?;

    handle.join().unwrap();
    Ok(())
}