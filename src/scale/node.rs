use log::*;
use tungstenite::{accept, Error, Message};
use uuid::Uuid;
use tokio::runtime::Runtime;

use std::{net::{TcpListener, TcpStream}, sync::{Arc, Mutex}};

use crate::{bombardier, parser, protocol::socket::WebSocketConn, report::stats, socket};

pub fn serve(port: &str) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let host = "127.0.0.1";
    let server = TcpListener::bind(format!("{}:{}", host, port))?;

    for stream in server.incoming() {
        std::thread::spawn(move || {   
                  
            let websocket = accept(stream.unwrap()).unwrap();
            let websocket_client = socket::WebSocketConn { 
                websocket : websocket,
                uuid: Uuid::new_v4().to_hyphenated().to_string()
            };
            
            let websocket_arc = Arc::new(Mutex::new(Some(websocket_client)));

            loop { 
                let message = match get_message(websocket_arc.clone()) {
                    Ok(m) => m,
                        Err(err) => {
                            error!("Error occured while readind the message {}", err);
                            return;
                        }
                };

                if message.is_close() {
                    info!("Distributor closed the connection");
                    return;
                }

                if message.is_text() {
                    //Currently only bombarding message is handled, any other message would result in an error
                    if handle_text_message(websocket_arc.clone(), message.to_text().unwrap()).is_err() { 
                        return;
                    }
                }
            }
        });
    }

    Ok(())
}


fn get_message(ws_client_arc: Arc<Mutex<Option<WebSocketConn<TcpStream>>>>) -> Result<Message, Error> {
    let mut ws_mtx_grd = ws_client_arc.lock().unwrap();
    ws_mtx_grd.as_mut().unwrap().read()
}

fn handle_text_message(websocket_clone: Arc<Mutex<std::option::Option<socket::WebSocketConn<std::net::TcpStream>>>>, text_message: &str) 
-> Result<(), Box<dyn std::error::Error + 'static>> {
//fn handle_text_message(websocket_client: &mut socket::WebSocketClient<TcpStream>, text_message: &str) -> Result<(), Box<dyn std::error::Error + 'static>> {
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

    //stats consumer
    let (stats_sender,  stats_receiver_handle) = stats::StatsConsumer::new(&bombardier.config, websocket_clone);

    //Bombard!!
    info!("Bombarding !!!");
    Runtime::new().unwrap().block_on(async {
        match bombardier.bombard(stats_sender).await {
            Err(err) => error!("Bombarding failed : {}", err),
            Ok(()) => ()
        } 
    });
    
    stats_receiver_handle.join().unwrap();
    
    info!("Bombarding Complete");
    Ok(())
}