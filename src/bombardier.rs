
use chrono::{Utc, DateTime};
use crossbeam::channel;
use log::{debug, error, info, warn, trace};
use serde::{Serialize, Deserialize};
use tokio::task::spawn;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread, 
    time,
    ops::Deref,
};

use crate::{
    cmd,
    file,
    model::scenarios,
    parse::{parser, postprocessor},
    protocol::http,
    report::stats,
};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct  Bombardier {
    pub config: cmd::ExecConfig,
    pub env_map: HashMap<String, String>,
    pub requests: Vec<scenarios::Request>,
    pub vec_data_map: Vec<HashMap<String, String>>
}

impl Bombardier {
    pub fn new(config: cmd::ExecConfig, env: String, scenarios: String, data: String) 
    -> Result<Bombardier, Box<dyn std::error::Error>>  {

        //prepare environment map
        info!("Parsing env map");
        let env_map = match parser::get_env_map(&env) {
            Err(err) => {
                error!("Error occured while parsing environments: {}", err);
                return Err(err);
            },
            Ok(map) => map
        };

        //prepare scenarios
        info!("Preparing bombardier attacks!");
        let requests = match parser::parse_requests(scenarios, &env_map) {
            Err(err) => {
                error!("Error occured while parsing requests : {}", err);
                return Err(err)
            },
            Ok(v) => v
        };

        //preparing data for attack
        info!("Parsing attack data");
        let vec_data_map = match parser::get_vec_data_map(data) {
            Err(err) => {
                error!("Error occured while parsing data  {}", err);
                return Err(Box::new(err))
            },
            Ok(vec) => vec
        };

        //preparing bombardier
        Ok(Bombardier {
            config,
            env_map,
            requests,
            vec_data_map
        })
    }
}

impl Bombardier {
    pub async fn bombard(&self, stats_sender: channel::Sender<Vec<stats::Stats>>)
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        
        let no_of_threads = self.config.thread_count;
        let no_of_iterations = self.config.iterations;
        let thread_delay = self.config.rampup_time * 1000 / no_of_threads;
        let think_time = self.config.think_time;
        let continue_on_error = self.config.continue_on_error;
    
        let client_arc = Arc::new(http::get_sync_client(&self.config)?);
        let requests = Arc::new(self.requests.clone());
       
        let data_count = self.vec_data_map.len();
        let vec_data_map_arc = Arc::new(self.vec_data_map.clone());
        let data_counter: usize = 0;
        let data_counter_arc = Arc::new(Mutex::new(data_counter));

        let stats_sender_arc = Arc::new(stats_sender.clone());
        
        let mut handles = vec![];

        let start_time = Utc::now();
        let execution_time = self.config.execution_time;

        for thread_cnt in 0..no_of_threads {
            let requests_clone = requests.clone();
            let client_clone = client_arc.clone();
            let mut env_map_clone = self.env_map.clone();
            let vec_data_map_clone = vec_data_map_arc.clone();
            let data_counter_clone = data_counter_arc.clone();
            let stats_sender_clone = stats_sender_arc.clone();

            let mut thread_iteration = 0;

            let handle = spawn(async move {
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
                        let mut data_couter_mg = data_counter_clone.lock().unwrap();
                        let data_map = get_data_map(&vec_data_map_clone, &mut data_couter_mg, data_count);
                        env_map_clone.extend(data_map.into_iter().map(|(k, v)| (k.clone(), v.clone())));
                        debug!("data used for {}-{} : {:?}", thread_cnt, thread_iteration, data_map);
                    }
                    
                    //looping thru requests
                    for request in requests_clone.deref() {
                        let processed_request = preprocess(&request, &env_map_clone); //transform request
                        trace!("Executing {}-{} : {}", thread_cnt, thread_iteration, serde_json::to_string_pretty(&processed_request).unwrap());

                        match http::execute(&client_clone, processed_request).await {
                            Ok((response, latency)) => {
                                let new_stats = stats::Stats::new(&request.name, response.status().as_u16(), latency);
                                vec_stats.push(new_stats); //Add stats to vector

                                if !continue_on_error && is_failed_request(response.status().as_u16()) { //check status
                                    warn!("Request {} failed with status {}. Skipping rest of the iteration", &request.name, response.status());
                                    break;
                                }

                                match postprocessor::process(response, &request, &mut env_map_clone).await { //process response and update env_map
                                    Err(err) => error!("Error occurred while post processing response for request {} : {}", &request.name, err),
                                    Ok(()) => ()
                                }
                            },
                            Err(err) => {
                                error!("Error occured while executing request {} : {}", &request.name, err);
                                if !continue_on_error {
                                    warn!("Skipping rest of the iteration as continue on error is set to false");
                                    break;
                                }
                            }
                        }
                        thread::sleep(time::Duration::from_millis(think_time)); //wait per request delay
                    };
                    
                    info!("Writing stats data");
                    stats_sender_clone.send(vec_stats.clone()).unwrap();
                }
            });

            handles.push(handle);
            thread::sleep(time::Duration::from_millis(thread_delay)); //wait per thread delay
        }

        futures::future::join_all(handles).await;
        
        drop(stats_sender);
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

fn preprocess(request: &scenarios::Request, env_map: &HashMap<String, String>) -> scenarios::Request {
    let mut s_request = serde_json::to_string(request).expect("Request cannot be serialized");
    s_request = file::param_substitution(s_request, &env_map);
    match serde_json::from_str(&s_request) {
        Ok(r) => r,
        Err(err) => {
            error!("Unable to deserialize request object after parameter replacement. Returning original request");
            error!("String: {}, Error: {}", s_request, err);
            request.clone()
        }
    }
}