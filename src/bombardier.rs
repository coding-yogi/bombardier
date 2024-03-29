use chrono::{Utc, DateTime};
use crossbeam::channel;
use log::{debug, info, error, warn};
use parking_lot::FairMutex as Mutex;
use reqwest::Request as Reqwest;
use serde::{Serialize, Deserialize};
use rustc_hash::FxHashMap as HashMap;
use tokio::{
    sync::{Mutex as TMutex},
    task::spawn,
    time
};

use std::{error::Error, sync::{Arc, atomic::{AtomicU16, Ordering}}};

use crate::{
    converter, 
    data::DataProvider, 
    model::*, 
    parse::{
        parser,
        preprocessor,
        postprocessor
    }, 
    protocol::http::{self, HttpClient}, 
    report::stats
};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct  Bombardier {
    pub config: Config,
    pub env_map: HashMap<String, String>,
    pub requests: Vec<Request>,
}

impl Bombardier {
    pub fn new(config: Config, env: String, scenarios: String) 
     -> Result<Bombardier, Box<dyn Error>>  {
        //Prepare environment map
        let env_map = match parser::parse_env_map(&env) {
            Err(err) => return Err(err),
            Ok(map) => map
        };
        
        //Prepare bombardier requests
        let requests = match parser::parse_requests(&scenarios, &env_map) {
            Err(err) => return Err(err),
            Ok(v) => v
        };

        //Preparing bombardier
        Ok(Bombardier {
            config,
            env_map,
            requests,
        })
    }
}

impl Bombardier {
    pub async fn bombard(&self, stats_sender: channel::Sender<Vec<stats::Stats>>)
    -> Result<(), Box<dyn Error + Send + Sync>> {
        //Setting execution config
        let no_of_iterations = self.config.iterations;
        let thread_delay = self.config.rampup_time * 1000 / self.config.thread_count;
        let think_time = self.config.think_time;
        let continue_on_error = self.config.continue_on_error;
        let execution_time = self.config.execution_time;
        let thread_count = self.config.thread_count;

        //set up data
        let data_provider = DataProvider::new(&self.config.data_file).await;
        let is_data_provided = data_provider.is_some();
        let data_provider_arc = Arc::new(TMutex::new(data_provider));
    
        //Set up client and requests
        let client = Arc::new(http::HttpClient::new(&self.config).await?);
        let requests = Arc::new(self.requests.to_owned());
       
        //Initiate Stats sender
        let stats_sender_arc = Arc::new(stats_sender.clone());

        //Initialize request cache
        let reqwest_cache = Arc::new(Mutex::new(HashMap::default()));
        
        let mut handles = vec![];
        let start_time = Utc::now();
        
        let threads_running = Arc::new(AtomicU16::new(0));
        
        for thread_cnt in 0..thread_count {
            info!("Starting thread: {}", thread_cnt+1);
            threads_running.fetch_add(1, Ordering::SeqCst);

            let requests = requests.clone();
            let client = client.clone();

            let mut env_map = self.env_map.clone(); //every thread will mutate this map as per runtime values
            let data_provider = data_provider_arc.clone();
            let stats_sender = stats_sender_arc.clone();
            let reqwest_cache = reqwest_cache.clone();
            let threads_running_clone = threads_running.clone();

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

                    //Update env map with data
                    if is_data_provided {
                        update_env_map_with_data(&mut env_map, data_provider.clone()).await;
                    }

                    //Initialize Stats vec
                    let mut vec_stats = Vec::with_capacity(requests.len());
                    
                    //looping thru requests
                    for request in requests.iter() {
                        let reqwest = match process_request(client.as_ref(), request, &env_map, reqwest_cache.clone()).await {
                            Ok(reqwest) => Some(reqwest),
                            Err(err) => {
                                error!("Error occured while processing request {} : {}", &request.name, err);
                                None
                            }
                        };

                        if reqwest.is_none() {
                            if continue_on_error {
                                continue;
                            } else {
                                break;
                            }
                        }

                        match client.execute(reqwest.unwrap()).await { //can safely unwrap as none is checked
                            Ok((response, latency)) => {
                                let status_code = response.status().as_u16();

                                match postprocessor::process(response, &request.extractors, &mut env_map).await { //process response and update env_map
                                    Err(err) => error!("Error occurred while post processing response for request {} : {}", &request.name, err),
                                    Ok(()) => ()
                                }

                                let new_stats = stats::Stats::new(&request.name, status_code, latency, threads_running_clone.load(Ordering::SeqCst));
                                vec_stats.push(new_stats); //Add stats to vector

                                if status_code > 399 { //check status
                                    info!("Request {} failed with status {}", &request.name, status_code);
                                    if !continue_on_error { 
                                        warn!("Skipping rest of the iteration as continueOnError is set to false");
                                        break;
                                    }
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

                        time::sleep(time::Duration::from_millis(think_time as u64)).await; //wait per request delay
                    };
                    
                    stats_sender.try_send(vec_stats).unwrap();
                }
            });

            handles.push(handle);
            time::sleep(time::Duration::from_millis(thread_delay as u64)).await; //wait per thread delay
        }

        futures::future::join_all(handles).await;
        
        drop(stats_sender);
        Ok(())
    }
}

async fn process_request(http_client: &HttpClient, request: &Request, env_map: &HashMap<String, String>, cache: Arc<Mutex<HashMap<uuid::Uuid, Reqwest>>>) 
-> Result<Reqwest, Box<dyn Error + Send + Sync>> {
    if request.requires_preprocessing {
        let processed_request = preprocessor::process(request, env_map); 
        return converter::convert_request(http_client, &processed_request).await
    } else {
        //Search the request in cache, if found return
        let reqwest = get_reqwest_from_cache(cache.clone(), &request.id);
        if reqwest.is_ok() {
            debug!("Using request {} from cache", request.name);
            return reqwest; 
        }

        debug!("Request {} not requiring post processing not found in cache", request.name);
        let reqwest = converter::convert_request(http_client, request).await?;

        //Try to add to cache if reqwest can be cloned, if not just return 
        if let Some(reqwest) = reqwest.try_clone() {
            debug!("Adding {} request to cache", request.name);
            let mut cache_guard = cache.lock();
            cache_guard.insert(request.id.to_owned(), reqwest);
            drop(cache_guard);
        } else {
            debug!("Failed adding request {} to cache", request.name);
        }

        Ok(reqwest)
    }
}

fn get_reqwest_from_cache(cache: Arc<Mutex<HashMap<uuid::Uuid, Reqwest>>>, id: &uuid::Uuid) -> Result<Reqwest, Box<dyn Error + Send + Sync>>{
    let cache_guard = cache.lock();
    if let Some(reqwest) = cache_guard.get(id) {
        Ok(reqwest.try_clone().unwrap())
    } else {
        Err("Could not clone the reqwest".into())
    }
}

async fn update_env_map_with_data(env_map: &mut HashMap<String, String>, data_provider: Arc<TMutex<Option<DataProvider>>>) {
    //get record
    let mut data_provider_mg = data_provider.lock().await; 
        
    if let Some(data_provider) = data_provider_mg.as_mut() {
        let data_map = data_provider.get_data().await;

        //update env map
        env_map.extend(data_map.into_iter());     
    }     
}

fn is_execution_time_over(start_time: DateTime<Utc>, duration: &u64) -> bool {
    (Utc::now().timestamp() - start_time.timestamp()) as u64 > *duration
}