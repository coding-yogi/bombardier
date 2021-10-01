use chrono::Utc;
use crossbeam::channel;
use log::{debug, error, info, warn};

use serde::{Serialize, Deserialize};
use tokio::{net::TcpStream, sync::Mutex, task};
use tokio_tungstenite::MaybeTlsStream;

use std::{
    fmt,
    option::Option,
    sync::Arc
};

use crate::{
    cmd::{self, Database},
    report::csv,
    socket, 
    storage::{self, DBWriter, influxdb}
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub timestamp: String,
    pub status: u16,
    pub latency: u128,
    pub name: String,
}

impl Stats {
    pub fn new(name: &str, status: u16, latency: u128) -> Stats {
        Stats {
            timestamp: Utc::now().to_rfc3339(),
            name: String::from(name),
            status,
            latency
        }
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}, {}, {}, {:width$}\n", self.timestamp, self.status, self.latency, self.name, width = 35)
    }
}

pub struct StatsConsumer {}

impl StatsConsumer {
    pub async fn new(config: &cmd::ExecConfig, websocket: 
        Arc<Mutex<Option<socket::WebSocketSink<MaybeTlsStream<TcpStream>>>>>) 
    -> Result<(channel::Sender<Vec<Stats>>, tokio::task::JoinHandle<()>),String> {

        info!("Initiate StatsConsumer");
        let (tx, rx) = channel::unbounded::<Vec<Stats>>();

        let is_distributed = config.distributed;

        //Initialize DB writer is DB is configured
        let db_writer = get_db_writer(&config.database);
        let is_db_configured = db_writer.is_some();
        let db_writer = Arc::new(Mutex::new(db_writer));

        //Initialize CSV Writer is execution is not distributed
        let mut opt_csv_writer = None;
        if !is_distributed {
            opt_csv_writer = match csv::CSVWriter::new(&cmd::DEFAULT_REPORT_FILE).await {
                Ok(w) => Some(w),
                Err(err) => return Err(err.to_string())
            };
        }

        let csv_writer = Arc::new(Mutex::new(opt_csv_writer));

        let stats_batch: Vec<Stats> = Vec::with_capacity(150);
        let stats_batch_arc = Arc::new(Mutex::new(stats_batch));

        let handle = task::spawn(async move {
            loop {
                match rx.recv() {
                    Ok(stats) => {
                        //Add stats to batch till batch size is full
                        let mut stats_batch = stats_batch_arc.lock().await;
                        stats_batch.extend(stats);

                        //Currently batch size is hardcoded to 100
                        if stats_batch.len() < 100 {
                            continue;
                        }

                        //need to drop this guard else below to async tasks cannot acquire a lock
                        drop(stats_batch); 

                        let csv_writer_clone = csv_writer.clone();
                        let websocket_clone = websocket.clone();
                        let stats_clone = stats_batch_arc.clone();

                        //Spawn a task for writing to CSV or socket
                        let handle1 = task::spawn(async move {

                            write_socket_or_csv(is_distributed, stats_clone, websocket_clone, csv_writer_clone).await;
                        });

                        let db_writer_clone = db_writer.clone();
                        let stats_clone = stats_batch_arc.clone();

                        //Spawn a task to write to DB
                        let handle2 = task::spawn(async move {
                            write_to_db(is_db_configured, stats_clone, db_writer_clone).await;
                        });

                        futures::future::join_all([handle1, handle2]).await;

                        //clear the batch
                        let mut stats_batch = stats_batch_arc.lock().await;
                        stats_batch.clear();
                    }
                    Err(err) => {       
                        if err.to_string().contains("receiving on an empty and disconnected channel") {
                            write_socket_or_csv(is_distributed, stats_batch_arc.clone(), websocket.clone(), csv_writer).await;

                            if is_distributed { 
                              send_done_to_websocket(websocket).await;
                            }

                            write_to_db(is_db_configured, stats_batch_arc, db_writer).await;
                            break;
                        }

                        //Log any other error apart from disconnected channel
                        error!("Error receiving msg on StatsConsumer channel: {}", err); 
                    }
                }
            }
        });

        Ok((tx, handle))
    }
}

async fn send_done_to_websocket(websocket: 
    Arc<Mutex<Option<socket::WebSocketSink<MaybeTlsStream<TcpStream>>>>>) {
    let mut websocket = websocket.lock().await;
    websocket.as_mut().unwrap().write(String::from("done")).await;
}

async fn write_socket_or_csv(is_distributed: bool, stats: Arc<Mutex<Vec<Stats>>>, websocket: 
    Arc<Mutex<Option<socket::WebSocketSink<MaybeTlsStream<TcpStream>>>>>, csv_writer: Arc<Mutex<Option<csv::CSVWriter>>>) {
    let stats = stats.lock().await;

    //check if distributed
    if is_distributed {
        //Sending stats data to hub over websocket connection
        let mut websocket = websocket.lock().await;
        websocket.as_mut().unwrap().write_stats(&stats[..]).await;
    } else {
        //Writing stats to CSV
        let mut csv_writer = csv_writer.lock().await;
        csv_writer.as_mut().unwrap().write(&stats[..]).await;
    }
}

async fn write_to_db(is_db_configured: bool, stats: Arc<Mutex<Vec<Stats>>>, db_writer: 
    Arc<Mutex<Option<Box<dyn DBWriter + Send>>>>) {
    if is_db_configured {
        let stats = stats.lock().await;
        let mut db_writer = db_writer.lock().await;
        db_writer.as_mut().unwrap().write_stats(&stats[..]).await;
    }
}

fn get_db_writer(db_config: &Database) -> Option<Box<dyn storage::DBWriter + Send>> {
    let db_writer;

    match db_config.db_type.to_lowercase().as_str() {
        "influxdb" => {
            info!("Initiating influx DB");
            match influxdb::InfluxDBWriter::new(&db_config) {
                Some(writer) =>  {
                    db_writer = Box::new(writer) as Box<dyn DBWriter + Send>;
                },
                None => {
                    error!("InfluxDB initialization failed");
                    return None
                }
            }
        },
        "" => {
            warn!("No database type defined. No DBWriter will be initialized");
            return None;
        },
        _ => {
            error!("Invalid DB type received: {}", &db_config.db_type);
            return None
        }
    }

    Some(db_writer)
}