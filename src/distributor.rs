use crate::cmd;
use crate::parser;
use crate::report;
use crate::socket;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use log::{debug, info};
use tungstenite::{connect, Message};

pub fn distribute(config: cmd::ExecConfig, env_map: HashMap<String, String>, requests: Vec<parser::Request>) -> Result<(), Box<dyn std::error::Error>> {

    let no_of_nodes = config.nodes.len();
    if no_of_nodes == 0 {
        return Err(Box::from("No nodes mentioned for distributed bombarding"));
    }

    info!("Creating report file: {}", &config.report_file);
    let report_file = report::create_file(&config.report_file)?;

    let message = socket::Message {
        config: config.clone(),
        env_map: env_map,
        requests: requests
    };

    let mut handles = vec![];
    let report_arc = Arc::new(Mutex::new(report_file));

    let mut sockets = vec![];
    
    //Make all connection first and abort if any connection fails
    for node in &config.nodes {
        let node_address = format!("ws://{}/ws", &node);
        let (socket, _) = connect(url::Url::parse(&node_address).unwrap())?;
        info!("Connected to {} successfully", &node_address);
        sockets.push(socket);
    }

    //Loop through all connections
    for mut socket in sockets {
        socket.write_message(Message::Text(serde_json::to_string(&message).unwrap().into()))?; //Send data to node

        let report_clone = report_arc.clone();
        handles.push(thread::spawn(move || {
            loop {
                let msg = socket.read_message().expect("Error reading message");
                
                if msg.is_text() { //Handle only text messages
                    let text_msg = msg.to_text().unwrap();
                    debug!("Received from node: {}", text_msg);

                    if text_msg == "done" { break; } //Exit once "done" message is received from node

                    //Write stats to CSV 
                    //TODO: Improve performance
                    let stats: Vec<report::Stats> = serde_json::from_str(text_msg).unwrap();
                    for stat in stats {
                        report::write_stats_to_csv(&mut report_clone.lock().unwrap(), &format!("{}", &stat.clone()));
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