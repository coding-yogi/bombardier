use chrono::Utc;
use crossbeam::channel;
use log::{error, info};

use serde::{Serialize, Deserialize};
use tokio::{net::TcpStream, sync::Mutex, task};
use tokio_tungstenite::MaybeTlsStream;

use std::{
    fmt,
    option::Option,
    sync::Arc
};

use crate::{
    cmd,
    report::{influxdb, csv},
    socket,
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
    -> (channel::Sender<Vec<Stats>>, tokio::task::JoinHandle<()>) {

        info!("Initiate StatsConsumer");
        let (tx, rx) = channel::unbounded::<Vec<Stats>>();
        
        let influx_writer = influxdb::InfluxDBWriter::new(&config.influxdb);
        let influx_writer_arc = Arc::new(Mutex::new(influx_writer));

        let is_distributed = config.distributed;
        info!("Is execution distributed: {}", is_distributed);

        let is_influxdb_configured = config.influxdb.url != "";
        info!("Is influx DB configured: {}", is_influxdb_configured);

        let opt_csv_writer;

        if is_distributed {
            opt_csv_writer = None
        } else {
            opt_csv_writer = Some(csv::CSVWriter::new(&config.report_file).await.unwrap());
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

                        //Sending data to influx DB if configured
                        if is_influxdb_configured {
                            let mut influx_writer_mg = influx_writer_arc.lock().await;
                            influx_writer_mg.as_mut().unwrap().write_stats(&stats[..]).await;
                        }
                    }
                    Err(err) => {       
                        //If distributed, channel has been closed explicitly, send done to hub
                        if is_distributed && err.to_string().contains("receiving on an empty and disconnected channel") { 
                            let mut ws_mtx_grd = websocket_arc.lock().await;
                            ws_mtx_grd.as_mut().unwrap().write(String::from("done")).await;
                            break;
                        }            
                        
                        //Error will be logged only in case of non-distributed execution as its not expected
                        error!("Error occured while receiving the stats message {}", err); 
                    }
                }
            }
        });

        (tx, handle)
    }
}