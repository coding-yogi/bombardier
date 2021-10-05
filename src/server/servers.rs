
use crossbeam::channel::Sender;
use log::*;
use rustc_hash::FxHashMap as HashMap;
use tokio::{
    sync::Mutex,
    task
};

use std::sync::Arc
;

use crate::{
    bombardier::Bombardier,
    server::hub::{rest, websocket}
};

pub struct Context {
    pub bombardiers_map: Arc<Mutex<HashMap<String, bool>>>,
    pub transmitters_map: Arc<Mutex<HashMap<String, Sender<Bombardier>>>>
}

impl Context {
    pub async fn get_total_nodes(&self) -> usize {  
        //Check if nodes are available
        info!("Checking whether nodes are available for execution");
        let trasmitter_map_mg = self.transmitters_map.lock().await;
        trasmitter_map_mg.len()  
    }
    
    pub async fn get_currently_bombarding_nodes(&self) -> usize {  
        //Check if nodes are available
        info!("Getting currently bombarding nodes");
        let bombardiers_map_mg = self.bombardiers_map.lock().await;
        bombardiers_map_mg.iter().filter(|&entry| *(entry).1).count()
    }
}

pub async fn serve(port: u16, ws_port: u16) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let context_arc = Arc::new(Context {
        bombardiers_map :  Arc::new(Mutex::new(HashMap::default())),
        transmitters_map: Arc::new(Mutex::new(HashMap::default()))
    });

    let context_clone = context_arc.clone();

    //Spawing websocket server into separate thread
    task::spawn(async move {
        match websocket::serve(ws_port, context_arc).await {
            Ok(_) => (),
            Err(err) => error!("error occurred in websocket connection {}", err)
        }
    });

    rest::serve(port, context_clone).await;

    Ok(())
}