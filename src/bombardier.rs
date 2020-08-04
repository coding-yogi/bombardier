use crate::cmd;
use crate::file;
use crate::http;
use crate::parser;
use crate::report;
use crate::influxdb;
use crate::postprocessor;
use crate::socket::WebSocketClient;

use crossbeam::crossbeam_channel as channel;
use chrono::{Utc, DateTime};
use log::{debug, error, warn, trace};
use parking_lot::FairMutex as Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::{thread, time};
use std::ops::Deref;

pub struct  Bombardier {
    pub config: cmd::ExecConfig,
    pub env_map: HashMap<String, String>,
    pub requests: Vec<parser::Request>,
    pub vec_data_map: Vec<HashMap<String, String>>
}

impl Bombardier {
    pub fn bombard(&self, ws_arc: Arc<Mutex<Option<WebSocketClient<std::net::TcpStream>>>>)
    -> Result<(), Box<dyn std::error::Error + 'static>> {

        let config = self.config.clone();

        let no_of_threads = config.thread_count;
        let no_of_iterations = config.iterations;
        let thread_delay = config.rampup_time * 1000 / no_of_threads;
        let think_time = config.think_time;
        let continue_on_error = config.continue_on_error;
        let is_distributed = config.distributed;
    
        let client_arc = Arc::new(http::get_sync_client(&config));
        let requests = Arc::new(self.requests.clone());
       
        let data_count = self.vec_data_map.len();
        let vec_data_map_arc = Arc::new(self.vec_data_map.clone());
        let data_counter: usize = 0;
        let data_counter_arc = Arc::new(Mutex::new(data_counter));

        //let csv_report_file = report::create_file(&config.report_file)?;
        let reporter = report::new(&config.report_file)?;
        
        let (csv_tx, csv_recv_handle) = init_csv_chan(reporter); //Start CSV channel
        let (report_tx, report_recv_handle) = init_report_chan(ws_arc, &config.influxdb); //Start reporting channel
        
        let mut handles = vec![];

        let start_time = Utc::now();
        let execution_time = config.execution_time;

        for thread_cnt in 0..no_of_threads {
            let requests_clone = requests.clone();
            let client_clone = client_arc.clone();
            let mut env_map_clone = self.env_map.clone();
            let vec_data_map_clone = vec_data_map_arc.clone();
            let data_counter_clone = data_counter_arc.clone();

            let csv_tx_clone = csv_tx.clone();
            let report_tx_clone = report_tx.clone();
            
            let mut thread_iteration = 0;

            let handle = thread::spawn(move || {
                loop {
                    if no_of_iterations > 0 { //Iteration Based execution
                        if thread_iteration >= no_of_iterations { 
                            break;
                        }
                    } else if is_execution_time_over(start_time, &execution_time) { //Time based execution
                        break;
                    }

                    thread_iteration += 1; //increment iteration
                    let mut vec_stats = vec![];

                    //get data set
                    if data_count > 0 {
                        let data_map = get_data_map(&vec_data_map_clone, &mut *data_counter_clone.lock(), data_count);
                        env_map_clone.extend(data_map.into_iter().map(|(k, v)| (k.clone(), v.clone())));
                        debug!("data used for {}-{} : {:?}", thread_cnt, thread_iteration, data_map);
                    }
                    
                    //looping thru requests
                    for request in requests_clone.deref() {
                        let processed_request = preprocess(&request, &env_map_clone); //transform request
                        trace!("Executing {}-{} : {}", thread_cnt, thread_iteration, serde_json::to_string_pretty(&processed_request).unwrap());

                        match http::execute(&client_clone, processed_request) {
                            Ok((response, latency)) => {
                                let new_stats = report::Stats::new(&request.name, response.status().as_u16(), latency);

                                if !is_distributed {
                                    csv_tx_clone.send(new_stats.clone()).unwrap(); //send stats to csv channel
                                }
                                
                                vec_stats.push(new_stats); //Add stats to vector

                                if !continue_on_error && is_failed_request(response.status().as_u16()) { //check status
                                    warn!("Request {} failed with status {}. Skipping rest of the iteration", &request.name, response.status());
                                    break;
                                }

                                match postprocessor::process(response, &request, &mut env_map_clone) { //process response and update env_map
                                    Err(err) => error!("Error occurred while post processing response for request {} : {}", &request.name, err),
                                    Ok(()) => ()
                                }
                            },
                            Err(err) => {
                                error!("Error occured while executing request {}, : {}", &request.name, err);
                                if !continue_on_error {
                                    warn!("Skipping rest of the iteration as continue on error is set to false");
                                    break;
                                }
                            }
                        }
                        thread::sleep(time::Duration::from_millis(think_time)); //wait per request delay
                    };
                    report_tx_clone.send(vec_stats.clone()).unwrap(); //Send data to distributor    
                }
            });

            handles.push(handle);
            thread::sleep(time::Duration::from_millis(thread_delay)); //wait per thread delay
        }

        for handle in handles {
            handle.join().unwrap();
        }

        drop(csv_tx);
        drop(report_tx);
       
        csv_recv_handle.join().unwrap();
        report_recv_handle.join().unwrap();

        Ok(())
    }
}

fn get_data_map<'a>(vec_data_map: &'a Vec<HashMap<String, String>>, counter: &mut usize, length: usize) -> &'a HashMap<String, String> {
    let data_map = &vec_data_map[*counter];
    *counter += 1;
    if *counter == length { *counter = 0; }
    &data_map
}

fn is_execution_time_over(start_time: DateTime<Utc>, duration: &u64) -> bool {
    (Utc::now().timestamp() - start_time.timestamp()) as u64 > *duration
}

fn is_failed_request(status: u16) -> bool {
    status > 399
}

fn preprocess(request: &parser::Request, env_map: &HashMap<String, String>) -> parser::Request {
    let mut s_request = serde_json::to_string(request).expect("Request cannot be serialized");
    s_request = file::find_and_replace(s_request, &env_map);
    match serde_json::from_str(&s_request) {
        Ok(r) => r,
        Err(err) => {
            error!("Unable to deserialize request object after parameter replacement. Returning original request");
            error!("String: {}, Error: {}", s_request, err);
            request.clone()
        }
    }
}

fn init_csv_chan(reporter: report::Reporter) -> (channel::Sender<report::Stats>, thread::JoinHandle<()>) {
    let (tx, rx): (channel::Sender<report::Stats>, channel::Receiver<report::Stats>) = channel::unbounded();
    let reporter_arc = Arc::new(Mutex::new(reporter));
    let handle = thread::spawn(move || {
        loop {
            match rx.recv() {
                Ok(stats) => reporter_arc.lock().write_stats_to_csv(&format!("{}", &stats)),
                Err(_) => break //Channel has been closed
            }
        }
    });

    (tx, handle)
}

fn init_report_chan(ws_arc: Arc<Mutex<Option<WebSocketClient<std::net::TcpStream>>>>, influxdb: &cmd::InfluxDB) 
-> (channel::Sender<Vec<report::Stats>>, thread::JoinHandle<()>) {

    let influxdb_client = influxdb::InfluxDBClient {client: http::get_default_sync_client(), influxdb: influxdb.clone()};
    let influx_arc = Arc::new(Mutex::new(influxdb_client));

    let (tx, rx): (channel::Sender<Vec<report::Stats>>, channel::Receiver<Vec<report::Stats>>) = channel::unbounded();

    let is_distributed = ws_arc.lock().is_some(); //socket must be some if distributed
    let write_to_influx = influxdb.url.starts_with("http") && influxdb.dbname.len() > 0;

    let handle = thread::spawn(move || { //Send stats to distributor
        let mut websocket_mg = ws_arc.lock();

        loop {
            match rx.recv() {
                Ok(stats) => {   
                    if write_to_influx {
                        influx_arc.lock().write_stats(stats.clone()); //write to influx
                    }
    
                    if is_distributed {
                        websocket_mg.as_mut().unwrap().write(serde_json::to_string(&stats).unwrap()); //write to websocket
                    }
                },
                Err(_) => {
                    if is_distributed { //If distributed, channel has been closed explicitly, send done to distributor
                        websocket_mg.as_mut().unwrap().write(String::from("done"));
                    }               
                    break;
                }
            }
        }
    });

    (tx, handle)
}