use crate::bombardier;
use crate::parser;
use crate::socket;

use std::net::TcpListener;

use log::{error, info};
use tungstenite::accept;

use std::sync::{Arc, Mutex};

pub fn serve(port: &str) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let host = "127.0.0.1";
    let server = TcpListener::bind(format!("{}:{}", host, port))?;

    for stream in server.incoming() {
        std::thread::spawn(move || {
            
            let websocket = accept(stream.unwrap()).unwrap();
            let websocket_arc = Arc::new(Mutex::new(Some(websocket)));
            let websocket_clone = websocket_arc.clone();

            loop {
                let raw_message = websocket_clone.lock().unwrap().as_mut().unwrap().read_message().unwrap();
                if raw_message.is_text() {
                    let message: socket::Message = match serde_json::from_str(&raw_message.to_text().unwrap()) { //Convert to socket message
                        Ok(m) => m,
                        Err(err) => {
                            error!("Error while deserializing tex to socket message: {}", err);
                            return;
                        }
                    };
            
                    //Read data file
                    let vec_data_map = match parser::get_vec_data_map(&message.config.data_file) {
                        Ok(vec) => vec,
                        Err(err) => {
                            error!("Error occured while reading data file {} : {}", &message.config.data_file, err);
                            return;
                        }
                    };
                    
                    //create an bombardier instance
                    let bombardier = bombardier::Bombardier {
                        config: message.config,
                        env_map: message.env_map,
                        requests: message.requests,
                        vec_data_map: vec_data_map
                    };

                    //Bombard!!
                    let websocket_clone2 = websocket_arc.clone();
                    match bombardier.bombard(websocket_clone2) {
                        Err(err) => error!("Bombarding failed : {}", err),
                        Ok(()) => ()
                    } 
                    
                    info!("Bombarding Complete");
                }
            }
        });
    }

    Ok(())
}