use chrono::{Utc, DateTime};
use crossbeam::channel;
use log::{error, info, warn, trace};
use serde::{Serialize, Deserialize};
use tokio::{
    sync::Mutex,
    task::spawn,
    time
};

use std::{
    collections::HashMap,
    error::Error,
    sync::Arc
};

use crate::{
    cmd, 
    model::*, 
    parse::{
        preprocessor,
        postprocessor
    }, 
    protocol::http, 
    report::{csv,stats}
};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct  Bombardier {
    pub config: cmd::ExecConfig,
    pub env_map: HashMap<String, String>,
    pub requests: Vec<Request>,
    pub vec_data_map: Vec<HashMap<String, String>>
}

impl Bombardier {
    pub async fn new(config: cmd::ExecConfig, env: String, scenarios: String, data: String) 
    -> Result<Bombardier, Box<dyn std::error::Error>>  {
        //Prepare environment map
        let env_map = match get_env_map(&env) {
            Err(err) => return Err(err),
            Ok(map) => map
        };
        
        //Prepare bombardier requests
        let requests = match parse_requests(scenarios, &env_map) {
            Err(err) => return Err(err),
            Ok(v) => v
        };

        //Prepare data for attack
        let vec_data_map = match get_vec_data_map(data).await {
            Err(err) => return Err(err),
            Ok(vec) => vec
        };

        //Preparing bombardier
        Ok(Bombardier {
            config,
            env_map,
            requests,
            vec_data_map
        })
    }
}

fn parse_requests(content: String, env_map: &HashMap<String, String>) -> Result<Vec<Request>, Box<dyn Error>> {
    info!("Preparing bombardier requests");
    let scenarios_yml = preprocessor::param_substitution(content, &env_map);

    let root: Root = match serde_yaml::from_str(&scenarios_yml) {
        Ok(r) => r,
        Err(err) => {
            error!("Parsing bombardier requests failed: {}", err.to_string());
            return Err(err.into())
        }
    };

    let mut requests = Vec::<Request>::new();
  
    for scenario in root.scenarios {
        for request in scenario.requests {
            requests.push(request);
        }
    } 

    Ok(requests)
}

fn get_env_map(content: &str) -> Result<HashMap<String, String>, Box<dyn Error>> {
    let mut env_map: HashMap<String, String> = HashMap::with_capacity(30);

    if content == "" {
        warn!("No environments data is being used for execution");
        return Ok(env_map);
    }

    info!("Parsing env map");
    let env: Environment = match serde_yaml::from_str(content) {
        Ok(e) => e,
        Err(err) => {
            error!("Parsing env content failed: {}", err.to_string());
            return Err(err.into())
        }
    };

    for var in env.variables {
        let key = var.0.as_str().unwrap().to_string();
        let value = var.1.as_str().unwrap().to_string();
        env_map.insert(key, value);
    }

    Ok(env_map)
}

async fn get_vec_data_map(data_content: String) -> Result<Vec<HashMap<String, String>>, Box<dyn Error>> {
    if data_content == "" {
        warn!("No external data is being used for execution");
        return Ok(Vec::<HashMap<String, String>>::new())
    }

    info!("Parsing attack data");
    let vec_data_map = 
        csv::CSVReader.get_records(data_content.as_bytes()).await;

    match vec_data_map {
        Ok(v) => Ok(v),
        Err(err) => {
            error!("Preparing attack data failed: {}", err.to_string());
            Err(err.into())
        }
    }
}

impl Bombardier {
    pub async fn bombard(&self, stats_sender: channel::Sender<Vec<stats::Stats>>)
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let no_of_iterations = self.config.iterations;
        let thread_delay = self.config.rampup_time * 1000 / self.config.thread_count;
        let think_time = self.config.think_time;
        let continue_on_error = self.config.continue_on_error;
    
        let client_arc = Arc::new(http::get_async_client(&self.config).await?);
        let requests = Arc::new(self.requests.to_owned());
       
        let vec_data_map_arc = Arc::new(self.vec_data_map.clone());
        let data_counter: usize = 0;
        let data_counter_arc = Arc::new(Mutex::new(data_counter));

        let stats_sender_arc = Arc::new(stats_sender.clone());
        
        let mut handles = vec![];

        let start_time = Utc::now();
        let execution_time = self.config.execution_time;

        for thread_cnt in 0..self.config.thread_count {
            let requests_clone = requests.clone();
            let client_clone = client_arc.clone();
            let mut env_map_clone = self.env_map.clone(); //every thread will mutate this map as per runtime values
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

                    //Update env map with data
                    update_env_map_with_data(&mut env_map_clone, &vec_data_map_clone, data_counter_clone.clone()).await;

                    //Initialize Stats vec
                    let mut vec_stats = Vec::with_capacity(requests_clone.len());
                    
                    //looping thru requests
                    for request in requests_clone.iter() {
                        let processed_request = preprocessor::process(request.to_owned(), &env_map_clone); //transform request
                        trace!("Executing {}-{} : {}", thread_cnt, thread_iteration, serde_json::to_string_pretty(&processed_request).unwrap());

                        match http::execute(&client_clone, &processed_request).await {
                            Ok((response, latency)) => {
                                let new_stats = stats::Stats::new(&request.name, response.status().as_u16(), latency);
                                vec_stats.push(new_stats); //Add stats to vector

                                if !continue_on_error && is_failed_request(response.status().as_u16()) { //check status
                                    warn!("Request {} failed with status {}. Skipping rest of the iteration", &request.name, response.status());
                                    break;
                                }

                                match postprocessor::process(response, &request.extractors, &mut env_map_clone).await { //process response and update env_map
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
                    
                    stats_sender_clone.try_send(vec_stats).unwrap();
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

async fn update_env_map_with_data(env_map: &mut HashMap<String, String>, vec_data_map: &Vec<HashMap<String, String>>, counter: Arc<Mutex<usize>>) {
    if vec_data_map.len() > 0 {
        //get one data set
        let mut data_couter_mg = counter.lock().await;
        let data_map = &vec_data_map[*data_couter_mg];

        //increment counter
        *data_couter_mg += 1;

        //If all data used, set it back to 0
        if *data_couter_mg == vec_data_map.len() { 
            *data_couter_mg = 0; 
        }

        drop(data_couter_mg);

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