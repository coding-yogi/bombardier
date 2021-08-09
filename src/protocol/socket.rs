
use log::*;
use serde::{Serialize, Deserialize};
use tungstenite::{
    protocol::WebSocket,
    connect as tconnect,
    Message,
};

use std::{
    collections::HashMap, 
    net::TcpStream
};

use crate::{cmd, model, report::stats};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BombardMessage {
    pub config: cmd::ExecConfig,
    pub env_map: HashMap<String, String>,
    pub requests: Vec<model::scenarios::Request>
}

pub struct WebSocketConn<T> {
    pub uuid: String,
    pub websocket: WebSocket<T>
}

impl <T> WebSocketConn<T> where T: std::io::Read + std::io::Write {
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

impl <T> stats::StatsWriter for WebSocketConn<T> where T: std::io::Read + std::io::Write {
    fn write_stats(&mut self, stats: &Vec<stats::Stats>) {
        self.write(serde_json::to_string(&stats).unwrap()) //check why json and not comma separated
    }
}

pub fn connect(url: String) -> Result<WebSocket<TcpStream>, Box<dyn std::error::Error>> {
     match tconnect(url::Url::parse(&url).unwrap()) {
         Ok((ws, _)) => Ok(ws),
         Err(err) => Err(format!("Connection failed: {}", err).into())
     }
}