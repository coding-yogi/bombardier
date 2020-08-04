use crate::cmd;
use crate::parser;
use crate::report;
use crate::socket;

use log::{debug, error, info};
use parking_lot::FairMutex as Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

pub fn distribute(config: cmd::ExecConfig, env_map: HashMap<String, String>, requests: Vec<parser::Request>) -> Result<(), Box<dyn std::error::Error>> {

    let no_of_nodes = config.nodes.len();
    if no_of_nodes == 0 {
        return Err(Box::from("No nodes mentioned for distributed bombarding"));
    }

    info!("Creating report file: {}", &config.report_file);
    let reporter = report::new(&config.report_file)?;
    let reporter_arc = Arc::new(Mutex::new(reporter));

    let mut sockets = HashMap::new();
    
    //Make all connection first and abort if any connection fails
    for node in &config.nodes {
        let node_address = format!("ws://{}/ws", &node);
        let websocket = socket::connect(node_address.clone())?;
        info!("Connected to {} successfully", &node_address);
        sockets.insert(node.clone(), socket::WebSocketClient { websocket });
    }

    let message = socket::BombardMessage {
        config: config.clone(),
        env_map: env_map,
        requests: requests
    };

    let mut handles = vec![];

    //Loop through all connections
    for (node, mut socket) in sockets {
        socket.write(serde_json::to_string(&message).unwrap()); //Send data to node
        let reporter_arc_clone = reporter_arc.clone();

        //let report_clone = report_arc.clone();
        handles.push(thread::spawn(move || {
            loop {
                let msg = match socket.read() {
                    Ok(m) => m,
                    Err(err) => return check_connection_error(err.to_string())
                };

                if msg.is_text() { //Handle only text messages
                    let text_msg = msg.to_text().unwrap();
                    debug!("Received from node: {}", text_msg);

                    if text_msg == "done" { //Exit once "done" message is received from node
                        info!("Done signal received from node {}. Closing connection", node);
                        socket.close();
                        return; 
                    } 

                    //Write stats to CSV - //TODO: Improve performance
                    let stats: Vec<report::Stats> = serde_json::from_str(text_msg).unwrap();
                    for stat in stats {
                        reporter_arc_clone.lock().write_stats_to_csv(&format!("{}", &stat.clone()));
                    }
                }
            } 
        }));
    }
       
    for handle in handles {
        handle.join().unwrap();
    }
    Ok(())
}

fn check_connection_error(error: String) {
    if !error.contains("Connection closed normally"){
        error!("Error occured while reading message: {}", error);
    }
}