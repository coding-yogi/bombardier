
use crossbeam::channel::Sender;
use log::*;
use tokio::task;

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, 
    }, 
    thread
};

use crate::{protocol::socket::BombardMessage, server::{rest, websocket}};

pub struct Context {
    pub bombardiers_map: Arc<Mutex<HashMap<String, bool>>>,
    pub transmitters_map: Arc<Mutex<HashMap<String, Sender<BombardMessage>>>>
}

pub async fn serve(port: u16, ws_port: u16) -> Result<(), Box<dyn std::error::Error + 'static>> {

    let context_arc = Arc::new(Context {
        bombardiers_map :  Arc::new(Mutex::new(HashMap::new())),
        transmitters_map: Arc::new(Mutex::new(HashMap::new()))
    });

    let context_clone = context_arc.clone();

    let mut handles = vec![];

    //Spawing websocket server
    handles.push(thread::spawn(move ||{
        match websocket::serve(ws_port, context_arc) {
            Ok(_) => (),
            Err(err) => error!("error occurred in websocket connection {}", err)
        }
    }));

    handles.push(thread::spawn(move ||{
        task::spawn(async move {
            rest::serve(port, context_clone).await;
        });
    }));

    // We may want to exist if any of the thread completes as there is no point in keeping other server running
    //TODO: ^^
    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}