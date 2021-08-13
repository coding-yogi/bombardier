use log::*;
use parking_lot::FairMutex as Mutex;
use uuid::Uuid;

use std::{
    collections::HashMap,
    sync::Arc,
    thread
};

use crate::{
    bombardier::Bombardier, 
    report::{
        csv, 
        stats,
        stats::StatsWriter
    }, 
    socket
};

pub fn distribute(bombardier: Bombardier) -> Result<(), Box<dyn std::error::Error>> {

    let no_of_nodes = bombardier.config.nodes.len();
    if no_of_nodes == 0 {
        return Err(Box::from("No nodes mentioned for distributed bombarding"));
    }

    info!("Creating report file: {}", &bombardier.config.report_file);
    let csv_writer = csv::CSVWriter::new(&bombardier.config.report_file).unwrap();
    let csv_writer_arc = Arc::new(Mutex::new(csv_writer));

    let mut sockets = HashMap::new();
    
    //Make all connection first and abort if any connection fails
    for node in &bombardier.config.nodes {
        let node_address = format!("ws://{}/ws", &node);
        let websocket = socket::connect(node_address.clone())?;
        info!("Connected to {} successfully", &node_address);
        sockets.insert(node.clone(), socket::WebSocketConn {   
            websocket : websocket,
            uuid: Uuid::new_v4().to_hyphenated().to_string() 
        });
    }

    let mut handles = vec![];

    //Loop through all connections
    for (node, mut socket) in sockets {
        socket.write(serde_json::to_string(&bombardier).unwrap()); //Send data to node
        let csv_writer_arc_clone = csv_writer_arc.clone();

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

                    //Write stats to CSV
                    let stats: Vec<stats::Stats> = serde_json::from_str(text_msg).unwrap();
                    csv_writer_arc_clone.lock().write_stats(&stats);
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