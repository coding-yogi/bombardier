use chrono::{Utc, DateTime};
use crossbeam::channel;
use log::{debug, error, warn};
use reqwest::Request as Reqwest;
use serde::{Serialize, Deserialize};
use tokio::{
    sync::Mutex,
    task::spawn,
    time
};
use uuid::Uuid;

use std::{
    error::Error,
    fs::File,
    collections::HashMap,
    sync::Arc
};

use crate::{
    cmd, 
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
    pub config: cmd::ExecConfig,
    pub env_map: HashMap<String, String>,
    pub requests: Vec<Request>,
}

impl Bombardier {
    pub fn new(config: cmd::ExecConfig, env: String, scenarios: String) 
     -> Result<Bombardier, Box<dyn Error>>  {
        //Prepare environment map
        let env_map = match parser::parse_env_map(&env) {
            Err(err) => return Err(err),
            Ok(map) => map
        };
        
        //Prepare bombardier requests
        let requests = match parser::parse_requests(scenarios, &env_map) {
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
    
        //Set up client and requests
        let client = Arc::new(http::HttpClient::new(&self.config).await?);
        let requests = Arc::new(self.requests.to_owned());
       
        //set up data
        let data_file = get_data_file(&self.config.data_file)?;
        let is_data_provided = data_file.is_some();

        let data_provider_arc;
        if let Some(file) = data_file {
            data_provider_arc = Arc::new(Mutex::new(Some(DataProvider::new(file).await)));
        } else {
            data_provider_arc = Arc::new(Mutex::new(None));
        }

        let stats_sender_arc = Arc::new(stats_sender.clone());
        
        let mut handles = vec![];

        let start_time = Utc::now();
        let execution_time = self.config.execution_time;

        for thread_cnt in 0..self.config.thread_count {
            let requests = requests.clone();
            let client = client.clone();
            let mut env_map = self.env_map.clone(); //every thread will mutate this map as per runtime values
            let data_provider = data_provider_arc.clone();
            let stats_sender = stats_sender_arc.clone();

            let mut thread_iteration = 0;

            let mut request_cache = HashMap::with_capacity(requests.len());

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
                        let reqwest = match process_request(client.as_ref(), &mut request_cache, &request, &env_map).await {
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

                        match client.execute(reqwest.unwrap()).await {
                            Ok((response, latency)) => {
                                let new_stats = stats::Stats::new(&request.name, response.status().as_u16(), latency);
                                vec_stats.push(new_stats); //Add stats to vector

                                if !continue_on_error && is_failed_request(response.status().as_u16()) { //check status
                                    warn!("Request {} failed with status {}. Skipping rest of the iteration", &request.name, response.status());
                                    break;
                                }

                                match postprocessor::process(response, &request.extractors, &mut env_map).await { //process response and update env_map
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
                        time::sleep(time::Duration::from_millis(think_time)).await; //wait per request delay
                    };
                    
                    stats_sender.try_send(vec_stats).unwrap();
                }
            });

            handles.push(handle);
            time::sleep(time::Duration::from_millis(thread_delay)).await; //wait per thread delay
        }

        futures::future::join_all(handles).await;
        
        drop(stats_sender);
        Ok(())
    }
}


async fn process_request(http_client: &HttpClient, cache: &mut HashMap<Uuid, Reqwest>, request: &Request, env_map: &HashMap<String, String>) 
-> Result<Reqwest, Box<dyn Error + Send + Sync>> {
    //check if request requires processing
    if !request.requires_preprocessing {
         //Search the request in cache, if found return
        if let Some(reqwest) = cache.get(&request.id) {
            debug!("Using request from cache");
            return Ok(reqwest.try_clone().unwrap()); //This would work safely if we were able to add reqwest to map
        } else {
            debug!("Request not requiring post processing not found in cache");
            let reqwest = match converter::convert_request(http_client, request).await {
                Ok(reqwest) => reqwest,
                Err(err) => {
                    error!("Cannot convert request into reqwest object");
                    return Err(err);
                }
            };

            //Try to add to cache if reqwest can be cloned, if not just return 
            if let Some(reqwest) = reqwest.try_clone() {
                debug!("Adding request to cache");
                cache.insert(request.id.to_owned(), reqwest);
            }
            
            return Ok(reqwest);
        }
    } else { //this request always needs processing so cannot be cached      
        debug!("Preprocessing request"); 
        let processed_request = preprocessor::process(request.to_owned(), &env_map); 
        converter::convert_request(http_client, &processed_request).await
    }
}


fn get_data_file(file_path: &str) -> Result<Option<File>, Box<dyn Error + Send + Sync>> {
    if file_path.trim() == "" {
        return Ok(None)
    }

    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(err) => {
            error!("Error while reading data file {}", err);
            return Err(err.into())
        }
    };

    Ok(Some(file))
}

async fn update_env_map_with_data(env_map: &mut HashMap<String, String>, data_provider: Arc<Mutex<Option<DataProvider<File>>>>) {
    //get record
    let mut data_provider_mg = data_provider.lock().await; 
        
    if let Some(data_provider) = data_provider_mg.as_mut() {
        let data_map = data_provider.get_data().await;
    
        //update env map
        env_map.extend(data_map.iter().map(|(k, v)| (k.clone(), v.clone())));    
    }     
}

fn is_execution_time_over(start_time: DateTime<Utc>, duration: &u64) -> bool {
    (Utc::now().timestamp() - start_time.timestamp()) as u64 > *duration
}

fn is_failed_request(status: u16) -> bool {
    status > 399
}