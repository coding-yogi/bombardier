use crossbeam::channel;
use futures::StreamExt;
use log::*;
use tokio_tungstenite::accept_async;
use tungstenite::{ Error, Message };
use uuid::Uuid;
use tokio::{
    net::{
        TcpListener,
        TcpStream
    },
    runtime,
    sync::Mutex
};

use std::{
    collections::HashMap,
    sync::{
        Arc 
    }
};

use crate::{
    protocol::
    socket::{
        self, 
        WebSocketSink, 
        WebSocketStream
    }, 
    server::servers
};

pub async fn serve(
    ws_port: u16,  ctx: Arc<servers::Context>) -> Result<(), Box<dyn std::error::Error + 'static>> {

    let host = "127.0.0.1";
    let server = match TcpListener::bind(format!("{}:{}", host, ws_port)).await {
        Ok(server) => server,
        Err(err) => {
            error!("Error starting websocket server {}", err);
            return Err(err.into());
        }
    };

    let runtime = runtime::Runtime::new().unwrap();

    loop {
        let conn_uuid = Uuid::new_v4().to_hyphenated().to_string();
        let conn_uuid1 = conn_uuid.clone();

        info!("awaiting new connection");
        let stream = server.accept().await.unwrap();
        let websocket =  accept_async(stream.0).await.unwrap();
        
        let(sink, stream) = websocket.split();

        let websocket_sink = Arc::new(Mutex::new(socket::WebSocketSink::new(sink)));
        let websocket_stream = Arc::new(Mutex::new(socket::WebSocketStream::new(stream)));

        info!("received connection with uuid {}", conn_uuid);
        
        //For every stream create a channel and add its transmitter to HashMap
        let (tx, rx) = channel::unbounded();
        add_to_map(ctx.transmitters_map.clone(), &conn_uuid, tx).await;

        let tansmitter_arc_clone = ctx.transmitters_map.clone();
        let bombardiers_arc_clone = ctx.bombardiers_map.clone();
        let bombardiers_arc_clone_2 = ctx.bombardiers_map.clone();

        let mut handles = vec![];

        handles.push(runtime.spawn(async move {   
            loop { 
                //Gets the incoming message from the node
                info!("awaiting response back from client");
                let message = match get_message(websocket_stream.clone()).await {
                    Ok(m) => m,
                    Err(err) => {
                        if err.to_string().contains("Connection reset without closing handshake") {
                            error!("Connection terminated by {}", &conn_uuid);
                            remove_from_map(tansmitter_arc_clone.clone(), &conn_uuid).await;
                            remove_from_map(bombardiers_arc_clone.clone(), &conn_uuid).await;
                        }
                        
                        return;
                    }
                };

                match message {
                    Message::Close(None) => {
                        info!("Connection with uuid {} was closed, removing transmitted and bombardiers from map", &conn_uuid);
                        remove_from_map(tansmitter_arc_clone.clone(), &conn_uuid).await;
                        remove_from_map(bombardiers_arc_clone.clone(), &conn_uuid).await;
                        return;
                    },

                    Message::Text(text) => {
                        match text.as_str() {
                            "Done" => {
                                info!("Received done from {}, updating bombardier to false", &conn_uuid);
                                add_to_map(bombardiers_arc_clone.clone(), &conn_uuid, false).await;
                            },
                            _ => ()
                        }
                    }

                    _ => ()
                }
            }
        }));

        //Spin another thread to listen on the receiver channel for bombard message
        handles.push(runtime.spawn(async move {
            loop {
                info!("awaiting for new message from rest server");
                match rx.recv() {
                    Ok(msg) => {
                        //Forward message to websocket connection
                        info!("Sending bombard message to {}", &conn_uuid1);
                        let message = serde_json::to_string(&msg).unwrap();
                        send_message(websocket_sink.clone(), message).await;

                        //update bombardiers map
                        info!("Updating status of node {} to bombarding", &conn_uuid1);
                        add_to_map(bombardiers_arc_clone_2.clone(), &conn_uuid1, true).await;
                    },
                    Err(err) => {
                        //handle error where its due to node being disconnected
                        if err.to_string().contains("receiving on an empty and disconnected channel") {
                            info!("channel for uuid {} is closed", &conn_uuid1);
                        } else {
                            error!("error occured while receiving message from rest server: {}", err);
                        }
                        
                        return;
                    }
                }
            }
        }));
    }
    //Ok(())
}

async fn add_to_map<A>(map_arc: Arc<Mutex<HashMap<String, A>>>, uuid: &str, val: A) {
    let mut map = map_arc.lock().await;
    map.insert(uuid.to_string(), val);
}

async fn remove_from_map<A>(map_arc: Arc<Mutex<HashMap<String, A>>>, uuid: &str) {
    let mut map = map_arc.lock().await;
    match map.remove_entry(uuid) {
        Some(_) => (),
        None => error!("value for uuid {} was not present in the map", uuid)
    }
}

//async fn get_message(ws_client_arc: Arc<Mutex<Option<WebSocketConn<TcpStream>>>>) -> Result<Message, Error> {
async fn get_message(ws_stream: Arc<Mutex<WebSocketStream<TcpStream>>>) -> Result<Message, Error> {
    let mut ws_mtx_grd = ws_stream.lock().await;
    ws_mtx_grd.read().await
}

async fn send_message(ws_client_arc: Arc<Mutex<WebSocketSink<TcpStream>>>, message: String) {
    let mut ws_mtx_grd = ws_client_arc.lock().await;
    ws_mtx_grd.write(message).await;
}