use crate::parser;
use crate::cmd;
use crate::http;

use std::sync::Arc;
use std::{thread, time};
use std::ops::Deref;

use log::{info, debug};

pub fn execute(args: cmd::Args, requests: Vec<parser::Request>) {

    let no_of_threads = args.threads;
    let no_of_iterations = args.iterations;
    let iteration_based_execution = no_of_iterations > 0;
    let thread_delay = calc_thread_delay(&args.threads, &args.ramp_up);
    info!("Thread delay calculated as {}", thread_delay);

    let start_time = time::Instant::now();
    let execution_time = args.execution_time;
   
    let client = http::get_sync_client();
    let client_arc = Arc::new(client);
    let args_arc = Arc::new(args);
    let requests = Arc::new(requests);

    let mut handles = vec![];

    for thread_cnt in 0..no_of_threads {
        let requests_clone = requests.clone();
        let client_clone = client_arc.clone();
        let args_clone = args_arc.clone();

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
                debug!("Executing thread {}-{}", thread_cnt, thread_iteration);

                //looping thru requests
                for request in requests_clone.deref() {
                    info!("Executing {}-{}  : {:?}", thread_cnt, thread_iteration, request.name);
                    http::execute(&client_clone, &request).unwrap();
                }
            }
        });

        handles.push(handle);

        //wait per thread delay
        thread::sleep(time::Duration::from_secs(thread_delay));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

fn calc_thread_delay(threads: &u64, rampup: &u64) -> u64 {
    rampup / threads
}

fn is_execution_time_over(start_time: time::Instant, duration: &u64) -> bool {
    start_time.elapsed().as_secs() > *duration
}