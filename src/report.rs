use crate::file;

use std::fmt;
use std::fs;
use std::process;
use std::io::Write;
use std::collections::HashSet;

use chrono::{Utc, DateTime};
use csv::Trim;
use log::{error, warn};
use prettytable::{Table, row, cell};
use rayon::prelude::*;
use serde::{Serialize, Deserialize};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    timestamp: String,
    name: String,
    status: u16,
    latency: u128
}

impl Stats {
    pub fn new(name: String, status: u16, latency: u128) -> Stats {
        Stats {
            timestamp: Utc::now().to_rfc3339(),
            name,
            status,
            latency
        }
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}, {}, {}, {:width$}\n", self.timestamp, self.status, self.latency, self.name, width = 35)
    }
}

pub fn create_file(path: &str) -> fs::File {
    let mut report_file = file::create_file(path);
    write_stats_to_csv(&mut report_file, &format!("timestamp, status, latency, name\n"));
    report_file
}

pub fn write_stats_to_csv(file: &mut fs::File, stat: &str) {
    match file.write(stat.as_bytes()) {
        Err(err) => warn!("Unable to write stat {} to file due to error {}", stat, err),
        Ok(_) => (),
    }
}

pub fn display(report_file: String) {

    let mut stats: Vec<Stats> = Vec::new();
    let mut names: HashSet<String> = HashSet::new();

    let rdr = csv::ReaderBuilder::new().has_headers(true).trim(Trim::All).from_path(report_file.as_str());
    match rdr {
        Ok(mut r) => {
            for stat in r.deserialize() {
                match stat {
                    Ok(s) => {
                        let s: Stats = s;
                        if !names.contains(&s.name) {
                            names.insert(s.name.clone());
                        }
                        stats.push(s);
                    },
                    Err(err) => {
                        error!("Unable to deserialize Stats: {}", err);
                        process::exit(-1);
                    }
                }
            }   
        },
        Err(err) => {
            error!("Unable to read report file {}", err);
            process::exit(-1);
        }
    }

    let mut table = Table::new();
    table.add_row(row![FY => "Request", "Total Hits", "Hits/s", "Min", "Avg", "Max", "90%", "95%", "99%", "Errors", "Error Rate"]);
     
    let mut total_hits = 0;
    let mut total_errors = 0.0;

    let starttime = DateTime::parse_from_rfc3339(&stats[0].timestamp).unwrap();
    let endtime = DateTime::parse_from_rfc3339(&stats[stats.len()-1].timestamp).unwrap();
    let et = endtime.signed_duration_since(starttime).num_seconds();

    for name in names {
        //HEAVY CLONING HAPPENING HERE - TRY TO FIX
        let filter: Vec<Stats> = stats.clone().into_iter().filter(|s| s.name == name).collect();
        let num = filter.len();

        let mut times: Vec<u128> = filter.par_iter().map(|s| s.latency).collect();
        times.sort();

        let min = times[0];
        let max = times[num-1];

        let pc_90 = get_percentile(&times, 90);
        let pc_95 = get_percentile(&times, 95);
        let pc_99 = get_percentile(&times, 99);

        let sum: u128 = filter.par_iter().map(|s| s.latency).sum();
        let sum = sum as usize;
        let avg: usize = sum/num;
        let num_f32 = num as f32;
        let et_f32 = et as f32;
        let tput: f32 = num_f32 / et_f32;
        let errors = filter.par_iter().filter(|s| s.status >= 400).count() as f32;
        let error_rate: f32 = errors * 100.0 / num_f32;

        table.add_row(row![&name, &num.to_string(), &tput.to_string(), &min.to_string(), 
                            &avg.to_string(), &max.to_string(), &pc_90.to_string(), &pc_95.to_string(), 
                            &pc_99.to_string(), &errors.to_string(), &error_rate.to_string()]);
        total_hits += num;
        total_errors += errors;
    }

    table.printstd();
    print_summary_table(et, total_hits, total_errors);
}

fn get_percentile(sorted_vector: &Vec<u128>, p: usize) -> u128 {
    let len = sorted_vector.len();
    match p*len/100 {
        0 => sorted_vector[0],
        _ => sorted_vector[(p*len/100)-1]
    }
}

fn print_summary_table(et: i64, total_hits: usize, total_errors: f32) {
    let mut sum_table = Table::new();
    sum_table.add_row(row![FG => "Total Execution Time (in secs)", "Total Hits", "Hits/s", "Total Errors", "Error Rate"]);

    let et = et as f32;
    let total_hits = total_hits as f32;
    let ttput =  total_hits/et ;
    let err_rate = total_errors * 100.0 / total_hits;

    sum_table.add_row(row![&et.to_string(), &total_hits.to_string(), &ttput.to_string(), &total_errors.to_string(), &err_rate.to_string()]);
    sum_table.printstd();
}