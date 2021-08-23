use std::sync::Arc;

use futures::StreamExt;
use log::{info, error};
use tokio::sync::Mutex;

use crate::{
    bombardier::Bombardier,
    protocol::socket::{self, WebSocketSink, WebSocketStream}, 
    stats::StatsConsumer
};

pub async fn start(hub_address: String) -> Result<(), Box<dyn std::error::Error + 'static>> {

    let hub_address = format!("ws://{}/ws", &hub_address);
    let websocket = socket::connect(hub_address.clone()).await?;
    let (sink, stream) = websocket.split();

    let mut websocket_stream = WebSocketStream::new(stream);
    let websocket_sink = WebSocketSink::new(sink);

    let sink_arc =  Arc::new(Mutex::new(Some(websocket_sink)));

    info!("Connected to {} successfully", &hub_address);

    loop {
        let msg = match websocket_stream.read().await {
            Ok(m) => m,
            Err(err) => {
                if err.to_string().contains("Trying to work with closed connection") {
                    error!("Connection closed by hub");
                    return Err(err.into()) ;
                } else {
                    error!("{}", &err.to_string());
                }

                continue;
            }
        };

        if msg.is_text() { //Handle only text messages
            let text_msg = msg.to_text().unwrap();

            let b = match is_bombard_message(text_msg)  {
                Some(b) => b,
                None =>  return Err("Bombarding message not received".into())
            };

            let (stats_sender,  stats_receiver_handle) = 
            StatsConsumer::new(&b.config,sink_arc.clone()).await;

            match b.bombard(stats_sender).await {
                Err(err) => error!("Bombarding failed : {}", err),
                Ok(()) => info!("Bombarding Complete. Run report command to get details")
            }
        
            stats_receiver_handle.await.unwrap();
        }
    } 
}

fn is_bombard_message(msg: &str) -> Option<Bombardier> {
    match serde_json::from_str(msg) {
        Ok(b) => b,
        Err(_) => None
    }
}