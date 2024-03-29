use chrono::Local;
use crossbeam::channel::{self, Sender, Receiver};
use log::{error, info, warn};
use serde::{Serialize, Deserialize};
use tokio::{net::TcpStream, sync::Mutex, task, task::JoinHandle};
use tokio_tungstenite::MaybeTlsStream;

use std::{fmt, option::Option, sync::Arc};

use crate::{
    model::{Database, Config},
    report::csv,
    protocol::socket::WebSocketSink, 
    storage::{self, DBWriter, influxdb}
};

use super::csv::CSVWriter;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub timestamp: String,
    pub thread_count: u16,
    pub status: u16,
    pub latency: u32,
    pub name: String,
}

impl Stats {
    pub fn new(name: &str, status: u16, latency: u32, thread_count: u16) -> Stats {
        Stats {
            timestamp: Local::now().to_string(),
            name: String::from(name),
            status,
            latency,
            thread_count
        }
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}, {}, {}, {}, {:width$}", self.timestamp, self.thread_count, self.status, self.latency, self.name, width = 35)
    }
}

pub struct StatsConsumer {
    is_distributed: bool,
    is_db_configured: bool,
    receiver: Receiver<Vec<Stats>>,
    db_writer: Arc<Mutex<Option<Box<dyn DBWriter + Send>>>>,
    csv_writer: Arc<Mutex<Option<CSVWriter>>>,
    websocket: Arc<Mutex<Option<WebSocketSink<MaybeTlsStream<TcpStream>>>>>
}

impl StatsConsumer {
    pub async fn new(config: &Config, websocket: Arc<Mutex<Option<WebSocketSink<MaybeTlsStream<TcpStream>>>>>) 
    -> Result<(Self,  Sender<Vec<Stats>>) ,String> {
        info!("Initiate StatsConsumer");
        let (sender, receiver) = channel::unbounded::<Vec<Stats>>();

        //Initialize DB writer is DB is configured
        let db_writer = get_db_writer(&config.database);
        let is_db_configured = db_writer.is_some();
        
        //Initialize CSV Writer if execution is not distributed
        let report_file;
        if config.report_file.is_empty() {
            report_file = "report.csv";
        } else {
            report_file = &config.report_file;
        }

        let mut csv_writer = None;
        let is_distributed = config.distributed;
        if !is_distributed {
            csv_writer = match csv::CSVWriter::new(report_file).await {
                Ok(w) => Some(w),
                Err(err) => return Err(err.to_string())
            };
        }

        Ok((StatsConsumer {
            is_distributed,
            is_db_configured,
            receiver,
            db_writer: Arc::new(Mutex::new(db_writer)),
            csv_writer: Arc::new(Mutex::new(csv_writer)),
            websocket
        }, sender))
    }
}

impl StatsConsumer {
    pub async fn consume(self) -> JoinHandle<()> {
        let is_db_configured = self.is_db_configured;
        let is_distributed = self.is_distributed;

        let db_writer = self.db_writer.clone();
        let csv_writer = self.csv_writer.clone();
        let websocket = self.websocket.clone();

        let stats_batch: Vec<Stats> = Vec::with_capacity(100);
        let stats_batch_arc = Arc::new(Mutex::new(stats_batch));

        task::spawn(async move {
            loop {
                match self.receiver.recv() {
                    Ok(stats) => {
                        //Add stats to batch till batch size is full
                        let mut stats_batch = stats_batch_arc.lock().await;
                        stats_batch.extend(stats);

                        //Currently batch size is hardcoded to 100
                        if stats_batch.len() < 50 {
                            continue;
                        }

                        //need to drop this guard else below two async tasks cannot acquire a lock
                        drop(stats_batch); 

                        let csv_writer_clone = csv_writer.clone();
                        let websocket_clone = websocket.clone();
                        let stats_clone = stats_batch_arc.clone();

                        //Spawn a task for writing to CSV or socket
                        let handle1 = task::spawn(async move {
                            if is_distributed {
                                write_to_socket(websocket_clone, stats_clone).await;
                            } else {
                                write_to_csv(csv_writer_clone, stats_clone).await;
                            }
                        });

                        let db_writer_clone = db_writer.clone();
                        let stats_clone = stats_batch_arc.clone();

                        //Spawn a task to write to DB
                        let handle2 = task::spawn(async move {
                            if is_db_configured {
                                write_to_db(db_writer_clone, stats_clone).await;
                            }
                        });

                        futures::future::join_all([handle1, handle2]).await;

                        //clear the batch
                        let mut stats_batch = stats_batch_arc.lock().await;
                        stats_batch.clear();
                    }
                    Err(err) => {       
                        if err.to_string().contains("receiving on an empty and disconnected channel") {
                            if is_distributed {
                                write_to_socket(websocket.clone(), stats_batch_arc.clone()).await;
                                send_done_to_websocket(websocket).await;
                            } else {
                                write_to_csv(csv_writer, stats_batch_arc.clone()).await;
                            }

                            if is_db_configured {
                                write_to_db(db_writer, stats_batch_arc).await;
                            }
                            
                            break;
                        }

                        //Log any other error apart from disconnected channel
                        error!("Error receiving msg on StatsConsumer channel: {}", err); 
                    }
                }
            }
        })
    }
}

async fn send_done_to_websocket(websocket: 
    Arc<Mutex<Option<WebSocketSink<MaybeTlsStream<TcpStream>>>>>) {
    let mut websocket = websocket.lock().await;
    websocket.as_mut().unwrap().write(String::from("done")).await;
}

async fn write_to_csv(csv_writer: Arc<Mutex<Option<csv::CSVWriter>>>, stats: Arc<Mutex<Vec<Stats>>>) {
    let stats = stats.lock().await;
    let mut csv_writer = csv_writer.lock().await;
    csv_writer.as_mut().unwrap().write(&stats[..]).await;  
}

async fn write_to_socket(websocket: 
    Arc<Mutex<Option<WebSocketSink<MaybeTlsStream<TcpStream>>>>>, stats: Arc<Mutex<Vec<Stats>>>) {
    let stats = stats.lock().await;
    let mut websocket = websocket.lock().await;
    websocket.as_mut().unwrap().write_stats(&stats[..]).await;
}

async fn write_to_db(db_writer: Arc<Mutex<Option<Box<dyn DBWriter + Send>>>>, stats: Arc<Mutex<Vec<Stats>>>) {
    let stats = stats.lock().await;
    let mut db_writer = db_writer.lock().await;
    db_writer.as_mut().unwrap().write_stats(&stats[..]).await;
}

fn get_db_writer(db_config: &Database) -> Option<Box<dyn storage::DBWriter + Send>> {
    let db_writer;

    match db_config.db_type.to_lowercase().as_str() {
        "influxdb" => {
            info!("Initiating influx DB");
            match influxdb::InfluxDBWriter::new(db_config) {
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