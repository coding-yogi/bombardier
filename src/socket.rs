use crate::cmd;
use crate::parser;

use std::collections::HashMap;

use log::{error};
use serde::{Serialize, Deserialize};
use tungstenite::protocol::{WebSocket};
use tungstenite::connect as tconnect;
use tungstenite::Message;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BombardMessage {
    pub config: cmd::ExecConfig,
    pub env_map: HashMap<String, String>,
    pub requests: Vec<parser::Request>
}

pub struct WebSocketClient<T> {
    pub websocket: WebSocket<T>
}

impl <T> WebSocketClient<T> where T: std::io::Read + std::io::Write {
    pub fn write(&mut self, message: String) {
        match self.websocket.write_message(Message::from(message)) {
            Ok(_) => (),
            Err(err) => error!("Error occured while writing to socket: {}", err)
        }
    }

    pub fn read(&mut self) -> Result<tungstenite::protocol::Message, tungstenite::error::Error> {
        self.websocket.read_message()
    }

    pub fn close(&mut self) {
        match self.websocket.write_message(Message::Close(None)) {
            Ok(_) => (),
            Err(err) => error!("Error occured while sending close message to socket: {}", err)
        }
    }
}

pub fn connect(url: String) -> Result<WebSocket<tungstenite::stream::Stream<std::net::TcpStream, native_tls::TlsStream<std::net::TcpStream>>>, Box<dyn std::error::Error>> {
     match tconnect(url::Url::parse(&url).unwrap()) {
         Ok((ws, _)) => Ok(ws),
         Err(err) => Err(format!("Connection failed: {}", err).into())
     }
}