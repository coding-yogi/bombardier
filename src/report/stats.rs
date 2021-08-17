use csv_async;
use chrono::{Utc, DateTime, Duration};
use crossbeam::channel;
use log::debug;
use prettytable::{Table, row, cell};
use rayon::prelude::*;
use serde::{Serialize, Deserialize};
use tokio::{fs::{self, File}, net::TcpStream, sync::Mutex, task};
use tokio_tungstenite::MaybeTlsStream;
use futures::StreamExt;

use std::{
    collections::HashSet,
    fmt,
    option::Option,
    sync::Arc
};

use crate::{
    cmd,
    file,
    protocol::http,
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

        let (tx, rx) = channel::unbounded();
        
        let influx_writer = influxdb::InfluxDBWriter::new(&config.influxdb, http::get_default_sync_client());
        let influx_writer_arc = Arc::new(Mutex::new(influx_writer));

        let is_distributed = config.distributed;
        let is_influxdb_configured = config.influxdb.url != "";

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
                        debug!("Received stats data");
                        if is_distributed {
                            let mut ws_mtx_grd = websocket_arc.lock().await;
                            ws_mtx_grd.as_mut().unwrap().write_stats(&stats).await;
                        } else {
                            let mut csv_mtx_grd = csv_writer_arc.lock().await;
                            csv_mtx_grd.as_mut().unwrap().write_stats(&stats).await;
                        }

                        if is_influxdb_configured {
                            let mut influx_writer_mg = influx_writer_arc.lock().await;
                            influx_writer_mg.as_mut().unwrap().write_stats(&stats).await;
                        }
                    }
                    Err(_) => {
                        if is_distributed { //If distributed, channel has been closed explicitly, send done to distributor
                            let mut ws_mtx_grd = websocket_arc.lock().await;
                            ws_mtx_grd.as_mut().unwrap().write(String::from("done")).await;
                        }               
                        break;
                    }
                }
            }
        });

        (tx, handle)
    }
}

pub async fn display(report_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = file::get_file(report_file).await?;
    let (names, stats) = get_stats(file).await?;

    let mut table = Table::new();
    table.add_row(row![FY => "Request", "Total Hits", "Hits/s", "Min", "Avg", "Max", "90%", "95%", "99%", "Errors", "Error Rate"]);
        
    let mut total_hits = 0;
    let mut total_errors = 0.0;

    let et = get_execution_time(&stats);

    for name in names {
        let name_filter: Vec<&Stats> = stats.par_iter().filter(|s| s.name == name).collect();
        let num = name_filter.len();

        let mut times: Vec<u128> = name_filter.par_iter().map(|s| s.latency).collect();
        times.sort();

        let (min, max) = (times[0], times[num-1]);
        let (pc_90, pc_95, pc_99) = get_all_percentiles(&times);

        let sum: usize = times.par_iter().sum::<u128>() as usize;
        let avg: usize = sum/num;
        let tput: f32 = num as f32 / et as f32;
        let errors = name_filter.par_iter().filter(|s| s.status >= 400).count() as f32;
        let error_rate: f32 = errors * 100.0 / num as f32;

        table.add_row(row![&name, &num.to_string(), &tput.to_string(), &min.to_string(), 
                            &avg.to_string(), &max.to_string(), &pc_90.to_string(), &pc_95.to_string(), 
                            &pc_99.to_string(), &errors.to_string(), &error_rate.to_string()]);
        total_hits += num;
        total_errors += errors;
    }

    table.printstd();
    print_summary_table(et, total_hits, total_errors);

    Ok(())
}

async fn get_stats(report_file: fs::File) -> Result<(HashSet<String>, Vec<Stats>), csv_async::Error> {
    let mut stats: Vec<Stats> = Vec::new();
    let mut names: HashSet<String> = HashSet::new();

    let mut reader: csv_async::AsyncReader<File> = csv_async::AsyncReaderBuilder::new()
        .has_headers(true)
        .trim(csv_async::Trim::All)
        .create_reader(report_file);

    let mut records_iter = reader.records();

    let header = csv_async::StringRecord::from(vec!["timestamp", "status", "latency", "name", ]);
    
    for stat in records_iter.next().await {
        let s: Stats = stat.unwrap().deserialize(Some(&header)).unwrap();
        if !names.contains(&s.name) {
            names.insert(s.name.clone());
        }
        stats.push(s);
    }   
    
    stats.sort_by_key(|k| k.timestamp.clone()); //required for distributed execution
    Ok((names, stats))
}

fn get_percentile(sorted_vector: &Vec<u128>, p: usize) -> u128 {
    let len = sorted_vector.len();
    match p*len/100 {
        0 => sorted_vector[0],
        _ => sorted_vector[(p*len/100)-1]
    }
}

fn get_all_percentiles(times: &Vec<u128>) -> (u128, u128, u128) {
    (get_percentile(&times, 90), get_percentile(&times, 95), get_percentile(&times, 99))
}

fn print_summary_table(et: i64, total_hits: usize, total_errors: f32) {
    let mut sum_table = Table::new();
    sum_table.add_row(row![FG => "Total Execution Time (in secs)", "Total Hits", "Hits/s", "Total Errors", "Error Rate"]);

    let et = et as f32;
    let total_hits = total_hits as f32;
    let ttput =  total_hits/et ;
    let err_rate = total_errors * 100.0 / total_hits;

    sum_table.add_row(row![&et.to_string(), &total_hits.to_string(), &ttput.to_string(), &total_errors.to_string(), &err_rate.to_string()]);
    sum_table.printstd();
}

fn get_execution_time(stats: &Vec<Stats>) -> i64 {
    let starttime = DateTime::parse_from_rfc3339(&stats[0].timestamp).unwrap() - Duration::milliseconds(stats[0].latency as i64);
    let endtime = DateTime::parse_from_rfc3339(&stats[stats.len()-1].timestamp).unwrap();
    endtime.signed_duration_since(starttime).num_seconds()
}