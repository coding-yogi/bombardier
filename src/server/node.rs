use futures::StreamExt;
use log::{info, error};

use crate::protocol::socket::{self, WebSocketStream};

pub async fn start(hub_address: String) -> Result<(), Box<dyn std::error::Error + 'static>> {

    let hub_address = format!("ws://{}/ws", &hub_address);
    let websocket = socket::connect(hub_address.clone()).await.unwrap();
    let (sink, stream) = websocket.split();

    let mut websocket_stream = WebSocketStream::new(stream);

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
            info!("Text message received from hub: {}", text_msg);
        }
    } 
}