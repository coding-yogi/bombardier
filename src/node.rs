use crate::bombardier;
use crate::parser;
use crate::socket;

use std::cell::Cell;
use std::io::{Error, ErrorKind};
use std::rc::Rc;

use log::{debug, error, info};
use ws::{
    Handler,
    Request,
    Response,
    Handshake,
    Sender,
    Message,
    Result,
    CloseCode,
    Error as WsError,
    listen
};

struct Node {
    sender: Sender,
    count: Rc<Cell<u32>>
}

impl Handler for Node {
    fn on_request(&mut self, req: &Request) -> Result<Response> {
        match req.resource() {
            "/ws" => {
                Response::from_request(req)
            },
            _ => Ok(Response::new(404, "Not found", b"404 - Not Found".to_vec())),
        }
    }

    fn on_open(&mut self, _: Handshake) -> Result<()> {
        self.count.set(self.count.get() + 1);
        let number_of_connection = self.count.get();

        if number_of_connection > 1 {
            error!("Node can accept only 1 request at a time");
            self.sender.close_with_reason(CloseCode::Policy, "Only single connection allowed")?;
            return Ok(());
        }

        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        let raw_message = msg.into_text()?;
        debug!("Message received from the client {:#?}", &raw_message);

        let message: socket::Message = match serde_json::from_str(&raw_message) {
            Ok(m) => m,
            Err(_) => return Err(Error::new(ErrorKind::InvalidData, "Unable to deserialize message from client").into())
        };


        let vec_data_map = match parser::get_vec_data_map(&message.config.data_file) {
            Ok(vec) => vec,
            Err(err) => return Err(Error::new(ErrorKind::InvalidData, format!("Error occured while reading data file {} : {}", &message.config.data_file, err)).into())
        };

        let bombardier = bombardier::Bombardier {
            config: message.config,
            env_map: message.env_map,
            requests: message.requests,
            vec_data_map: vec_data_map,
            ws: Some(self.sender.clone())
        };

        match bombardier.bombard() {
            Err(err) => error!("Bombarding failed : {}", err),
            Ok(()) => ()
        } 
        
        info!("Execution Complete");
        Ok(())
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        match code {
            CloseCode::Normal => info!("The client is done with the connection."),
            CloseCode::Away => info!("The client is leaving the site."),
            CloseCode::Abnormal => {
                info!("Closing handshake failed! Unable to obtain closing status from client.")
            },
            _ => info!("The client encountered an error: {}", reason),
        }
        self.count.set(self.count.get() - 1)
    }

    fn on_error(&mut self, err: WsError) {
        error!("The server encountered an error: {:?}", err);
    }
}

pub fn serve(port: &str) -> () {
    let host = "127.0.0.1";
    info!("Web Socket Server is ready at ws://{}:{}/ws",host, port);
    let count = Rc::new(Cell::new(0));
    listen(format!("{}:{}", host, port), |sender| { 
        Node { 
            sender: sender, 
            count: count.clone()
        } 
    }).unwrap()
}