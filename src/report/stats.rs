use chrono::Utc;
use crossbeam::channel;
use log::{error, info, warn};

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
    pub name: String,
    pub status: u16,
    pub latency: u128
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
    pub async fn new(config: &cmd::ExecConfig, websocket_arc: 
        Arc<Mutex<Option<socket::WebSocketSink<MaybeTlsStream<TcpStream>>>>>) 
    -> Result<(channel::Sender<Vec<Stats>>, tokio::task::JoinHandle<()>),String> {

        info!("Initiate StatsConsumer");
        let (tx, rx) = channel::unbounded::<Vec<Stats>>();

        let is_distributed = config.distributed;

        //Initialize DB writer is DB is configured
        let db_writer = get_db_writer(&config.database);
        let is_db_configured = db_writer.is_some();
        let db_writer_arc = Arc::new(Mutex::new(db_writer));

        //Initialize CSV Writer is execution is not distributed
        let mut opt_csv_writer = None;
        if !is_distributed {
            opt_csv_writer = match csv::CSVWriter::new(&cmd::DEFAULT_REPORT_FILE).await {
                Ok(w) => Some(w),
                Err(err) => return Err(err.to_string())
            };
        }

        let csv_writer_arc = Arc::new(Mutex::new(opt_csv_writer));

        let handle = task::spawn(async move {
            loop {
                match rx.recv() {
                    Ok(stats) => {
                        //check if distributed
                        if is_distributed {
                            //Sending stats data to hub over websocket connection
                            let mut ws_mtx_grd = websocket_arc.lock().await;
                            ws_mtx_grd.as_mut().unwrap().write_stats(&stats[..]).await;
                        } else {
                            //Writing stats to CSV
                            let mut csv_mtx_grd = csv_writer_arc.lock().await;
                            csv_mtx_grd.as_mut().unwrap().write(&stats[..]).await;
                        }

                        //Sending data to DB if configured
                        if is_db_configured {
                            let mut db_writer_mg = db_writer_arc.lock().await;
                            db_writer_mg.as_mut().unwrap().write_stats(&stats[..]).await;
                        }
                    }
                    Err(err) => {       
                        if err.to_string().contains("receiving on an empty and disconnected channel") {
                            //If distributed, send done from Node
                            if is_distributed { 
                                let mut ws_mtx_grd = websocket_arc.lock().await;
                                ws_mtx_grd.as_mut().unwrap().write(String::from("done")).await;
                            } 

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