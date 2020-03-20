use crate::cmd;
use crate::file;
use crate::http;
use crate::parser;
use crate::report;

use std::sync::{Arc, Mutex};
use std::{thread, time};
use std::ops::Deref;
use std::collections::HashMap;

use log::{debug, info};
use reqwest::{blocking::Response};

pub fn execute(args: cmd::Args, env_map: HashMap<String, String>, requests: Vec<parser::Request>) -> Vec<report::Stats> {

    let no_of_threads = args.threads;
    let no_of_iterations = args.iterations;
    let iteration_based_execution = no_of_iterations > 0;
    let thread_delay = args.ramp_up * 1000 / no_of_threads;

    let start_time = time::Instant::now();
    let execution_time = args.execution_time;
   
    let client = http::get_sync_client(&args);
    let client_arc = Arc::new(client);
    let args_arc = Arc::new(args);
    let requests = Arc::new(requests);

    let mut handles = vec![];
    let stats = vec![];
    let stats_arc = Arc::new(Mutex::new(stats));

    for thread_cnt in 0..no_of_threads {
        let requests_clone = requests.clone();
        let client_clone = client_arc.clone();
        let args_clone = args_arc.clone();
        let stats_clone = stats_arc.clone();
        let mut env_map_clone = env_map.clone();

        let mut thread_iteration = 0;
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

                //looping thru requests
                for request in requests_clone.deref() {
                    debug!("Executing {}-{} : {}", thread_cnt, thread_iteration, request.name);

                    let transformed_request = transform(&request, &env_map_clone); //transform request
                    match http::execute(&client_clone, transformed_request) {
                        Ok((res, et)) => {
                            stats_clone.lock().unwrap().push(report::Stats::new(request.name.clone(), res.status().as_u16(), et)); //push stats
                            update_env_map(res, &mut env_map_clone) //process response and update env_map
                        },
                        _ => ()
                    };

                    thread::sleep(time::Duration::from_millis(args_clone.delay)); //wait per request delay
                }
            }
        });

        handles.push(handle);
        thread::sleep(time::Duration::from_millis(thread_delay)); //wait per thread delay
    }

    for handle in handles {
        handle.join().unwrap();
    }

    match Arc::try_unwrap(stats_arc) {
        Ok(r) =>  r.into_inner().unwrap(),
        Err(_) => panic!("Unable to get report object")
    }
}

fn is_execution_time_over(start_time: time::Instant, duration: &u64) -> bool {
    start_time.elapsed().as_secs() > *duration
}

fn update_env_map(response: Response, env_map: &mut HashMap<String, String>) {
    let resp_body = response.text();
}

fn transform(request: &parser::Request, env_map: &HashMap<String, String>) -> parser::Request {
    let mut s_request = serde_json::to_string(request).expect("Request cannot be serialized");
    s_request = file::find_and_replace(s_request, &env_map);
    let transformed_request: parser::Request = serde_json::from_str(&s_request).expect("Unable to parse Json");
    transformed_request
}