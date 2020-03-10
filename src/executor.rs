use crate::parser;
use crate::cmd;

use std::sync::Arc;
use std::{thread, time};
use std::ops::Deref;

use log::{info, debug};

pub fn execute(args: cmd::Args, requests: Vec<parser::BombardierRequest>) {

    let thread_delay = calc_thread_delay(&args.threads, &args.ramp_up);
    let requests = Arc::new(requests);
    let mut handles = vec![];

    let start_time = time::Instant::now();
    let execution_time = args.execution_time;
    let execution_time = Arc::new(execution_time);

    for thread_cnt in 0..args.threads {
        let requests_clone = requests.clone();
        let execution_time_clone = execution_time.clone();
        let mut thread_iteration = 0;
        let handle = thread::spawn(move || {
            loop {
                //Continue next iteration if execution time hasn't passed
                if is_execution_time_over(start_time, execution_time_clone.deref()) {
                    break;
                }

                thread_iteration += 1; //increment iteration
                debug!("Executing thread {}-{}", thread_cnt, thread_iteration);

                //looping thru requests
                for request in requests_clone.deref() {
                    debug!("Executing {}-{}-{:?}", thread_cnt, thread_iteration,request);
                
                    //Delay between 2 requests
                    thread::sleep(time::Duration::from_millis(500));
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