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
use tokio::runtime::Builder;
use ws::Sender;

pub struct Bombardier {
    pub config: cmd::ExecConfig,
    pub env_map: HashMap<String, String>,
    pub requests: Vec<parser::Request>,
    pub vec_data_map: Vec<HashMap<String, String>>,
    pub ws: Option<Sender>
}

impl Bombardier {
    pub fn bombard(&self)
    -> Result<(), Box<dyn std::error::Error + 'static>> {

        let config = self.config.clone();

        let no_of_threads = config.thread_count;
        let no_of_iterations = config.iterations;
        let iteration_based_execution = no_of_iterations > 0;
        let thread_delay = config.rampup_time * 1000 / no_of_threads;

        let start_time = time::Instant::now();
        let execution_time = self.config.execution_time;
    
        let client = http::get_sync_client(&config);
        let client_arc = Arc::new(client);

        let influx_client = http::get_async_client();
        let influx_req = influxdb::build_request(&influx_client, &config.influxdb);
        let influx_arc = Arc::new(influx_req);

        let is_distributed = config.distributed;
        let report_file;
        if is_distributed {
            report_file = None;
        } else {
            report_file = Some(report::create_file(&config.report_file)?);
        } 

        let sender_arc = Arc::new(self.ws.clone());
        let args_arc = Arc::new(config);
        let requests = Arc::new(self.requests.clone());

        let mut handles = vec![];
        let report_arc = Arc::new(Mutex::new(report_file));

        let data_count = self.vec_data_map.len();
        let vec_data_map_arc = Arc::new(self.vec_data_map.clone());
        let data_counter: usize = 0;
        let data_counter_arc = Arc::new(Mutex::new(data_counter));

        for thread_cnt in 0..no_of_threads {
            let requests_clone = requests.clone();
            let client_clone = client_arc.clone();
            let influx_clone = influx_arc.clone();
            let args_clone = args_arc.clone();
            let mut env_map_clone = self.env_map.clone();
            let report_clone = report_arc.clone();
            let vec_data_map_clone = vec_data_map_arc.clone();
            let data_counter_clone = data_counter_arc.clone();
            
            let sender_clone = sender_arc.clone();
            
            let mut thread_iteration = 0;
            let rt = Builder::new().threaded_scheduler().enable_all().build()?;

            let handle = thread::spawn(move || {
                loop {
                    if iteration_based_execution {
                        if thread_iteration >= no_of_iterations {
                            break;
                        }
                    } else if is_execution_time_over(start_time, &execution_time) {
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
                                let new_stats_clone = new_stats.clone();
                                let report_clone2 = report_clone.clone();
                                let sender_clone2 = sender_clone.clone();
                                
                                let handle;

                                if is_distributed {
                                    handle = task::spawn(async move { //Send stats to distributor
                                        let payload = serde_json::to_string(&new_stats_clone.clone()).unwrap();
                                        match sender_clone2.clone().as_ref().as_ref().unwrap().send(payload) {
                                            Err(_) => error!("Unable to send stats to distributor"),
                                            Ok(_) => ()
                                        };
                                    });
                                } else {
                                    handle = task::spawn(async move {  //Write to CSV
                                       // let file: std::fs::File = report_clone2.as_ref().lock().unwrap().as_mut();
                                        report::write_stats_to_csv(&mut report_clone2.as_ref().lock().unwrap().as_mut().unwrap(), &format!("{}", &new_stats_clone.clone()));
                                    });
                                }
                            
                                vec_stats.push(new_stats);

                                if !args_clone.continue_on_error && is_failed_request(response.status().as_u16()) { //check status
                                    warn!("Request {} failed. Skipping rest of the iteration", &request.name);
                                    task::block_on(async {handle.await}); //wait for csv writing or sending stats to distributor
                                    break;
                                }

                                match postprocessor::process(response, &request, &mut env_map_clone) { //process response and update env_map
                                    Err(err) => error!("Error occurred while post processing response for request {} : {}", &request.name, err),
                                    Ok(()) => ()
                                }

                                task::block_on(async {handle.await}); //wait for csv writing or sending stats to distributor
                                
                            },
                            Err(err) => {
                                error!("Error occured while executing request {}, : {}", &request.name, err);
                                if !args_clone.continue_on_error {
                                    warn!("Skipping rest of the iteration as continue on error is set to false");
                                    break;
                                }
                            }
                        }

                        thread::sleep(time::Duration::from_millis(args_clone.thread_delay)); //wait per request delay
                    }

                    let builder_clone = influx_clone.deref().try_clone();
                    match builder_clone {
                        None => (),
                        Some(b) => {
                            rt.spawn(async {
                                influxdb::write_stats(b, vec_stats).await; //Write to influxDB
                            });
                        },
                    };
                }

                thread::sleep(time::Duration::from_millis(500)); //Tempfix for influxdb tasks to finish
            });

            handles.push(handle);
            thread::sleep(time::Duration::from_millis(thread_delay)); //wait per thread delay
        }

        for handle in handles {
            handle.join().unwrap();
        }

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
