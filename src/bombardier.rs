use crate::cmd;
use crate::file;
use crate::http;
use crate::parser;
use crate::report;
use crate::influxdb;
use crate::postprocessor;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, mpsc};
use std::{thread, time};
use std::ops::Deref;

use log::{debug, error, warn, trace};
use tungstenite::protocol::WebSocket;

pub struct  Bombardier {
    pub config: cmd::ExecConfig,
    pub env_map: HashMap<String, String>,
    pub requests: Vec<parser::Request>,
    pub vec_data_map: Vec<HashMap<String, String>>
}

impl Bombardier {
    pub fn bombard(&self, ws_arc: Arc<Mutex<Option<WebSocket<std::net::TcpStream>>>>)
    -> Result<(), Box<dyn std::error::Error + 'static>> {

        let config = self.config.clone();

        let no_of_threads = config.thread_count;
        let no_of_iterations = config.iterations;
        let thread_delay = config.rampup_time * 1000 / no_of_threads;
        let think_time = config.think_time;
        let continue_on_error = config.continue_on_error;
        let is_distributed = config.distributed;
        let write_to_influx = config.influxdb.url.starts_with("http");

        let start_time = time::Instant::now();
        let execution_time = config.execution_time;
    
        let client_arc = Arc::new(http::get_sync_client(&config));
        let requests = Arc::new(self.requests.clone());
       
        let data_count = self.vec_data_map.len();
        let vec_data_map_arc = Arc::new(self.vec_data_map.clone());
        let data_counter: usize = 0;
        let data_counter_arc = Arc::new(Mutex::new(data_counter));

        let csv_report_file = report::create_file(&config.report_file)?;
        
        let (csv_tx, csv_recv_handle) = init_csv_chan(csv_report_file); //Start CSV channel
        let (ws_tx, ws_recv_handle) = init_ws_chan(ws_arc); //Start websockets channel

        let (influx_tx, influx_recv_handle) =  init_influxdb_chan(&config.influxdb); //Start influx DB channel

        let mut handles = vec![];

        for thread_cnt in 0..no_of_threads {
            let requests_clone = requests.clone();
            let client_clone = client_arc.clone();
            let mut env_map_clone = self.env_map.clone();
            let vec_data_map_clone = vec_data_map_arc.clone();
            let data_counter_clone = data_counter_arc.clone();

            let csv_tx_clone = csv_tx.clone();
            let ws_tx_clone = ws_tx.clone();
            let influx_tx_clone = influx_tx.clone();
            
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
                        let data_map = get_data_map(&vec_data_map_clone, &mut *data_counter_clone.lock().unwrap(), data_count);
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
                                    warn!("Request {} failed. Skipping rest of the iteration", &request.name);
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
                    }

                    if is_distributed {
                        ws_tx_clone.send(vec_stats.clone()).unwrap(); //Send data to distributor
                    }

                    if write_to_influx {
                        influx_tx_clone.send(vec_stats).unwrap(); //send data to influx  channel    
                    }       
                }

                thread::sleep(time::Duration::from_millis(500)); //Tempfix for influxdb tasks to finish
            });

            handles.push(handle);
            thread::sleep(time::Duration::from_millis(thread_delay)); //wait per thread delay
        }

        for handle in handles {
            handle.join().unwrap();
        }

        drop(csv_tx);
        drop(ws_tx);
        drop(influx_tx);

        csv_recv_handle.join().unwrap();
        ws_recv_handle.join().unwrap();
        influx_recv_handle.join().unwrap();

        Ok(())
    }
}

fn get_data_map<'a>(vec_data_map: &'a Vec<HashMap<String, String>>, counter: &mut usize, length: usize) -> &'a HashMap<String, String> {
    let data_map = &vec_data_map[*counter];
    *counter += 1;
    if *counter == length { *counter = 0; }
    &data_map
}

fn is_execution_time_over(start_time: time::Instant, duration: &u64) -> bool {
    start_time.elapsed().as_secs() > *duration
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

fn init_csv_chan(file: std::fs::File) -> (mpsc::Sender<report::Stats>, thread::JoinHandle<()>) {
    let (tx, rx): (mpsc::Sender<report::Stats>, mpsc::Receiver<report::Stats>) = mpsc::channel();
    let file_arc = Arc::new(Mutex::new(file));
    let handle = thread::spawn(move || {
        loop {
            match rx.recv() {
                Ok(stats) => report::write_stats_to_csv(&mut file_arc.lock().unwrap(), &format!("{}", &stats)),
                Err(err) => {
                    if err.to_string().contains("receiving on a closed channel") {
                        break;
                    } else {
                        error!("Error occured on receiver: {}", err);
                    }
                }
            }
        }
    });

    (tx, handle)
}

fn init_influxdb_chan(influxdb: &cmd::InfluxDB) -> (mpsc::Sender<Vec<report::Stats>>, thread::JoinHandle<()>) {
    let influx_req = influxdb::build_request(&http::get_default_sync_client(), influxdb);
    let influx_arc = Arc::new(Mutex::new(influx_req));
    let (tx, rx): (mpsc::Sender<Vec<report::Stats>>, mpsc::Receiver<Vec<report::Stats>>) = mpsc::channel();
    
    let handle = thread::spawn(move || {
        loop {
            match rx.recv() {
                Ok(stats) => {
                    let builder_clone = influx_arc.lock().unwrap().deref().try_clone().unwrap();
                    influxdb::write_stats(builder_clone, stats.clone());
                }
                Err(err) => {
                    if err.to_string().contains("receiving on a closed channel") {
                        break;
                    } else {
                        error!("Error occured on receiver: {}", err);
                    }
                }
            }
        }
    });

    (tx, handle)
}

fn init_ws_chan(websocket: Arc<Mutex<Option<WebSocket<std::net::TcpStream>>>>) 
-> (mpsc::Sender<Vec<report::Stats>>, thread::JoinHandle<()>) {
    let (tx, rx): (mpsc::Sender<Vec<report::Stats>>, mpsc::Receiver<Vec<report::Stats>>) = mpsc::channel();

    let handle = thread::spawn(move || { //Send stats to distributor
        loop {
            match rx.recv() {
                Ok(stats) => {
                    let message = tungstenite::Message::from(serde_json::to_string(&stats).unwrap());
                    if write_to_websocket(websocket.clone(), message).is_err() {
                        break;
                    };
                },
                Err(err) => {
                    if err.to_string().contains("receiving on a closed channel") {
                        write_to_websocket(websocket.clone(), tungstenite::Message::from("done")).unwrap();
                        break;
                    } else {
                        error!("Error occured on receiver: {}", err);
                    }
                }
            }
        }
    });

    (tx, handle)
}

fn write_to_websocket(websocket: Arc<Mutex<Option<WebSocket<std::net::TcpStream>>>>, message: tungstenite::Message) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut socket = websocket.lock().unwrap();
    if socket.is_none() {
        return Err("socket is none".into())
    }

    match socket.as_mut().unwrap().write_message(message) {
        Err(_) => error!("Unable to send stats to distributor"),
        Ok(_) => ()
    }

    Ok(())
}