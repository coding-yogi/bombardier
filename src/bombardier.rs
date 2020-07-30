use crate::cmd;
use crate::file;
use crate::http;
use crate::parser;
use crate::report;
use crate::influxdb;
use crate::postprocessor;

use async_std::task;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
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

        let start_time = time::Instant::now();
        let execution_time = config.execution_time;
    
        let client_arc = Arc::new(http::get_sync_client(&config));

        let influx_req = influxdb::build_request(&http::get_async_client(), &config.influxdb);
        let influx_arc = Arc::new(influx_req);

        let requests = Arc::new(self.requests.clone());
        
        let report_file = get_report_file(is_distributed, &config.report_file);
        let report_arc = Arc::new(Mutex::new(report_file));
       
        let data_count = self.vec_data_map.len();
        let vec_data_map_arc = Arc::new(self.vec_data_map.clone());
        let data_counter: usize = 0;
        let data_counter_arc = Arc::new(Mutex::new(data_counter));

        let mut handles = vec![];
        
        for thread_cnt in 0..no_of_threads {
            let requests_clone = requests.clone();
            let client_clone = client_arc.clone();
            let influx_clone = influx_arc.clone();
            let mut env_map_clone = self.env_map.clone();
            let report_clone = report_arc.clone();
            let vec_data_map_clone = vec_data_map_arc.clone();
            let data_counter_clone = data_counter_arc.clone();
            
            let ws_clone = ws_arc.clone();
            
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
                                
                                let mut handle = None;
                                if !is_distributed {
                                    handle = Some(write_to_csv(new_stats.clone(), report_clone.clone()));
                                }
                                let handle_arc = Arc::new(Mutex::new(handle));
                            
                                vec_stats.push(new_stats); //Add stats to vector

                                if !continue_on_error && is_failed_request(response.status().as_u16()) { //check status
                                    warn!("Request {} failed. Skipping rest of the iteration", &request.name);
                                    wait_for_write_to_csv(handle_arc.clone()); //wait for csv writing or sending stats to distributor
                                    break;
                                }

                                match postprocessor::process(response, &request, &mut env_map_clone) { //process response and update env_map
                                    Err(err) => error!("Error occurred while post processing response for request {} : {}", &request.name, err),
                                    Ok(()) => ()
                                }

                                wait_for_write_to_csv(handle_arc.clone()); //wait for csv writing or sending stats to distributor   
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
                        write_to_socket(vec_stats.clone(), ws_clone.clone()); //Send data to distributor
                    }

                    write_to_influxdb(vec_stats, influx_clone.clone()); //Write to influx            
                }

                thread::sleep(time::Duration::from_millis(500)); //Tempfix for influxdb tasks to finish
            });

            handles.push(handle);
            thread::sleep(time::Duration::from_millis(thread_delay)); //wait per thread delay
        }

        for handle in handles {
            handle.join().unwrap();
        }

        if is_distributed {
            ws_arc.clone().lock().unwrap().as_mut().unwrap().write_message(tungstenite::Message::from("done"))?;
        }

        Ok(())
    }
}

fn get_report_file(is_distributed: bool, path: &str) -> Option<std::fs::File> {
    if is_distributed {
        None
    } else {
        Some(report::create_file(path).unwrap())
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

fn write_to_csv(stats: report::Stats, file: std::sync::Arc<std::sync::Mutex<std::option::Option<std::fs::File>>>) -> task::JoinHandle<()>{
    task::spawn(async move {  //Write to CSV
        report::write_stats_to_csv(&mut file.as_ref().lock().unwrap().as_mut().unwrap(), &format!("{}", &stats.clone()));
    })
}

fn wait_for_write_to_csv(handle: std::sync::Arc<std::sync::Mutex<std::option::Option<task::JoinHandle<()>>>>) {
    task::block_on(async { 
        if handle.lock().unwrap().as_mut().is_some() {
            handle.lock().unwrap().as_mut().unwrap().await
        }    
    });
}

fn write_to_influxdb(stats: Vec<report::Stats>, influxdb: std::sync::Arc<reqwest::RequestBuilder>) {
    let builder_clone = influxdb.deref().try_clone();
    match builder_clone {
        None => (),
        Some(b) => {
            task::spawn(async {
                influxdb::write_stats(b, stats).await; 
            });
        },
    };
}

fn write_to_socket(stats: Vec<report::Stats>, websocket: std::sync::Arc<std::sync::Mutex<std::option::Option<tungstenite::protocol::WebSocket<std::net::TcpStream>>>>) -> task::JoinHandle<()> {
    task::spawn(async move { //Send stats to distributor
        let payload = serde_json::to_string(&stats).unwrap();
        let mut socket = websocket.lock().unwrap();
        if socket.is_some() {
            match socket.as_mut().unwrap().write_message(tungstenite::Message::from(payload)) {
                Err(_) => error!("Unable to send stats to distributor"),
                Ok(_) => ()
            };
        }
    })
}