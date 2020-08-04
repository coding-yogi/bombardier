use crate::bombardier;
use crate::parser;
use crate::socket;

use log::{error, info};
use parking_lot::FairMutex as Mutex;
use std::net::TcpListener;
use std::sync::Arc;
use tungstenite::accept;
use parking_lot::lock_api;

pub fn serve(port: &str) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let host = "127.0.0.1";
    let server = TcpListener::bind(format!("{}:{}", host, port))?;

    for stream in server.incoming() {
        std::thread::spawn(move || {         
            let websocket = accept(stream.unwrap()).unwrap();
            let websocket_client = socket::WebSocketClient { websocket };
            let websocket_arc = Arc::new(Mutex::new(Some(websocket_client)));
            let websocket_clone = websocket_arc.clone();

            loop { 
                let raw_message = websocket_clone.lock().as_mut().unwrap().read().unwrap();
                
                if raw_message.is_close() {
                    info!("Distributor closed the connection");
                    return;
                }

                if raw_message.is_text() {
                    //Currently only bombarding message is handled, any other message would result in an error
                    if handle_text_message(websocket_clone.clone(), raw_message.to_text().unwrap()).is_err() { 
                        return;
                    }
                }
            }
        });
    }

    Ok(())
}

fn handle_text_message(websocket_clone: Arc<lock_api::Mutex<parking_lot::RawFairMutex, std::option::Option<socket::WebSocketClient<std::net::TcpStream>>>>, text_message: &str) 
-> Result<(), Box<dyn std::error::Error + 'static>> {
    let message: socket::BombardMessage = match serde_json::from_str(&text_message) { //Convert to socket message
        Ok(m) => m,
        Err(err) => {
            error!("Error while deserializing text to socket message: {}", err);
            return Err(err.into());
        }
    };

    //Read data file
    let vec_data_map = match parser::get_vec_data_map(&message.config.data_file) {
        Ok(vec) => vec,
        Err(err) => {
            error!("Error occured while reading data file {} : {}", &message.config.data_file, err);
            return Err(err.into());
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
    info!("Bombarding !!!");
    match bombardier.bombard(websocket_clone) {
        Err(err) => error!("Bombarding failed : {}", err),
        Ok(()) => ()
    } 
    
    info!("Bombarding Complete");
    Ok(())
}