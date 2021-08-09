
use std::sync::{Arc, Mutex};

use crate::{protocol::
    socket::{
        WebSocketConn
    }
};

pub fn handle_text_message(socket: Arc<Mutex<std::option::Option<WebSocketConn<std::net::TcpStream>>>>, text_message: &str) 
-> Result<(), Box<dyn std::error::Error + 'static>> {
    Ok(())
}