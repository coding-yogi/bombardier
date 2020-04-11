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


use log::{debug, error, warn};
use tokio::runtime::Builder;

pub fn bombard(args: cmd::Args, env_map: HashMap<String, String>, requests: Vec<parser::Request>) {

    let no_of_threads = args.thread_count;
    let no_of_iterations = args.iterations;
    let iteration_based_execution = no_of_iterations > 0;
    let thread_delay = args.rampup_time * 1000 / no_of_threads;

    let start_time = time::Instant::now();
    let execution_time = args.execution_time;

    let report_file = report::create_file(&args.report_file);
   
    let client = http::get_sync_client(&args);
    let client_arc = Arc::new(client);

    let influx_client = http::get_async_client();
    let influx_req = influxdb::build_request(&influx_client, &args.influxdb);
    let influx_arc = Arc::new(influx_req);

    let args_arc = Arc::new(args);
    let requests = Arc::new(requests);

    let mut handles = vec![];
    let report_arc = Arc::new(Mutex::new(report_file));

    for thread_cnt in 0..no_of_threads {
        let requests_clone = requests.clone();
        let client_clone = client_arc.clone();
        let influx_clone = influx_arc.clone();
        let args_clone = args_arc.clone();
        let mut env_map_clone = env_map.clone();
        let report_clone = report_arc.clone();
        
        let mut thread_iteration = 0;
        let rt = Builder::new().threaded_scheduler().enable_all().build().unwrap();

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
            
                //looping thru requests
                for request in requests_clone.deref() {
                    let processed_request = preprocess(&request, &env_map_clone); //transform request
                    debug!("Executing {}-{} : {}", thread_cnt, thread_iteration, serde_json::to_string_pretty(&processed_request).unwrap());
                    match http::execute(&client_clone, processed_request) {
                        Ok((response, latency)) => {
                            let new_stats = report::Stats::new(&request.name, response.status().as_u16(), latency);
                            let new_stats_clone = new_stats.clone();
                            let report_clone2 = report_clone.clone();

                            //Write to CSV
                            let write_csv_handle = task::spawn(async move {
                                report::write_stats_to_csv(&mut report_clone2.as_ref().lock().unwrap(), &format!("{}", &new_stats_clone));
                            });

                            vec_stats.push(new_stats);

                            //check status
                            if !args_clone.continue_on_error && is_failed_request(response.status().as_u16()) {
                                warn!("Request {} failed. Skipping rest of the iteration", &request.name);
                                task::block_on(async {write_csv_handle.await}); //wait for csv writing
                                break;
                            }

                            match postprocessor::process(response, &request, &mut env_map_clone) { //process response and update env_map
                                Err(err) => error!("Error occurred while post processing response for request {} : {}", &request.name, err),
                                Ok(()) => ()
                            }

                            task::block_on(async {write_csv_handle.await}) //for for csv writing
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
