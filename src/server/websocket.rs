use crossbeam::channel;
use log::*;
use tungstenite::{accept, Error, Message};
use uuid::Uuid;
use tokio::runtime;

use std::{
    collections::HashMap,
    net::{
        TcpListener, TcpStream
    }, 
    sync::{
        Arc, Mutex, 
    }
};

use crate::{
    protocol::socket::{
        self,
        WebSocketConn
    },
    server::servers
};

pub fn serve(
    ws_port: u16,  ctx: Arc<servers::Context>) -> Result<(), Box<dyn std::error::Error + 'static>> {

    let host = "127.0.0.1";
    let server = match TcpListener::bind(format!("{}:{}", host, ws_port)) {
        Ok(server) => server,
        Err(err) => {
            error!("Error starting websocket server {}", err);
            return Err(err.into());
        }
    };

    for stream in server.incoming() {
        let conn_uuid = Uuid::new_v4().to_hyphenated().to_string();
        let conn_uuid1 = conn_uuid.clone();

        let websocket_conn = socket::WebSocketConn { 
            uuid: conn_uuid.clone(),
            websocket: accept(stream.unwrap()).unwrap() 
        };

        info!("received connection with uuid {}", conn_uuid);

        let websocket_arc = Arc::new(Mutex::new(Some(websocket_conn)));
        
        //For every stream create a channel and add its transmitter to HashMap
        let (tx, rx) = channel::unbounded();
        add_to_map(ctx.transmitters_map.clone(), &conn_uuid, tx);

        let websocket_arc_clone = websocket_arc.clone();
        let tansmitter_arc_clone = ctx.transmitters_map.clone();
        let bombardiers_arc_clone = ctx.bombardiers_map.clone();
        let bombardiers_arc_clone_2 = ctx.bombardiers_map.clone();

        let mut handles = vec![];

        let runtime = runtime::Runtime::new().unwrap();

        handles.push(runtime.spawn(async move {   

            

            loop { 
                //Gets the incoming message from the connection
                let message = match get_message(websocket_arc_clone.clone()) {
                    Ok(m) => m,
                    Err(err) => {
                        error!("Error occured while reading the message {}", err);
                        return;
                    }
                };

                match message {
                    Message::Close(None) => {
                        info!("Connection with uuid {} was closed, removing transmitted and bombardiers from map", &conn_uuid);
                        remove_from_map(tansmitter_arc_clone.clone(), &conn_uuid);
                        remove_from_map(bombardiers_arc_clone.clone(), &conn_uuid);
                        return;
                    },

                    Message::Text(text) => {
                        match text.as_str() {
                            "Done" => {
                                info!("Received done from {}, updating bombardier to false", &conn_uuid);
                                add_to_map(bombardiers_arc_clone.clone(), &conn_uuid, false);
                            },
                            _ => ()
                        }
                    }

                    _ => ()
                }
            }
        }));

        //Spin another thread to listen on the receiver channel for bombard message //TODO: Try async
        handles.push(runtime.spawn(async move {
            loop {
                match rx.recv() {
                    Ok(msg) => {
                        //Forward message to websocket connection
                        info!("Sending bombard message to {}", &conn_uuid1);
                        let message = serde_json::to_string(&msg).unwrap();
                        send_message(websocket_arc.clone(), message);

                        //update bombardiers map
                        add_to_map(bombardiers_arc_clone_2.clone(), &conn_uuid1, true);
                    },
                    Err(err) => {
                        error!("error occured while receiving bombarding message from rest server {}", err);
                        return;
                    }
                }
            }
        }));

        runtime.block_on(async {
            futures::future::join_all(handles).await;
        });
    }

    Ok(())
}

fn add_to_map<A>(map_arc: Arc<Mutex<HashMap<String, A>>>, uuid: &str, val: A) {
    let mut map = map_arc.lock().unwrap();
    map.insert(uuid.to_string(), val);
}

fn remove_from_map<A>(map_arc: Arc<Mutex<HashMap<String, A>>>, uuid: &str) {
    let mut map = map_arc.lock().unwrap();
    match map.remove_entry(uuid) { //remove from transmitter map
        Some(_) => (),
        None => error!("value for uuid {} was not present in the map", uuid)
    }
}

fn get_message(ws_client_arc: Arc<Mutex<Option<WebSocketConn<TcpStream>>>>) -> Result<Message, Error> {
    let mut ws_mtx_grd = ws_client_arc.lock().unwrap();
    ws_mtx_grd.as_mut().unwrap().read()
}

fn send_message(ws_client_arc: Arc<Mutex<Option<WebSocketConn<TcpStream>>>>, message: String) {
    let mut ws_mtx_grd = ws_client_arc.lock().unwrap();
    ws_mtx_grd.as_mut().unwrap().write(message);
}