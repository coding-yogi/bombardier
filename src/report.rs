pub mod csv;
pub mod stats;

use chrono::{DateTime, Duration};
use prettytable::{Table, row, cell};
use rayon::prelude::*;
use std::fs::File;

use std::collections::HashSet;

use crate::report::stats::Stats;
use crate::data;

pub async fn display(report_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(report_file)?;
    let (names, stats) = get_stats(file).await?;

    let mut table = Table::new();
    table.add_row(row![FY => "Request", "Total Hits", "Hits/s", "Min", "Avg", "Max", "90%", "95%", "99%", "Errors", "Error Rate"]);
        
    let mut total_hits = 0;
    let mut total_errors = 0.0;

    let et = get_execution_time(&stats);

    for name in names {
        let name_filter: Vec<&Stats> = stats.par_iter().filter(|s| s.name == name).collect();
        let num = name_filter.len();

        let mut times: Vec<u128> = name_filter.par_iter().map(|s| s.latency).collect();
        times.sort_unstable();

        let (min, max) = (times[0], times[num-1]);
        let (pc_90, pc_95, pc_99) = get_all_percentiles(&times);

        let sum: usize = times.par_iter().sum::<u128>() as usize;
        let avg: usize = sum/num;
        let tput: f32 = num as f32 / et as f32;
        let errors = name_filter.par_iter().filter(|s| s.status >= 400).count() as f32;
        let error_rate: f32 = errors * 100.0 / num as f32;

        table.add_row(row![&name, &num.to_string(), &tput.to_string(), &min.to_string(), 
                            &avg.to_string(), &max.to_string(), &pc_90.to_string(), &pc_95.to_string(), 
                            &pc_99.to_string(), &errors.to_string(), &error_rate.to_string()]);
        total_hits += num;
        total_errors += errors;
    }

    table.printstd();
    print_summary_table(et, total_hits, total_errors);

    Ok(())
}

async fn get_stats(report_file: std::fs::File) -> Result<(HashSet<String>, Vec<Stats>), Box<dyn std::error::Error>> {
    let mut data_provider = data::DataProvider::new(report_file).await;
    let mut stats: Vec<Stats> = data_provider.get_records_as().await.unwrap();

    let names = stats.iter().map(|s| {
        s.name.clone()
    }).collect();
    
    stats.sort_by_key(|k| k.timestamp.clone()); //required for distributed execution
    Ok((names, stats))
}

fn get_percentile(sorted_vector: &[u128], p: usize) -> u128 {
    let len = sorted_vector.len();
    match p*len/100 {
        0 => sorted_vector[0],
        _ => sorted_vector[(p*len/100)-1]
    }
}

fn get_all_percentiles(times: &[u128]) -> (u128, u128, u128) {
    (get_percentile(times, 90), get_percentile(times, 95), get_percentile(times, 99))
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

fn get_execution_time(stats: &[Stats]) -> i64 {
    let starttime = DateTime::parse_from_rfc3339(&stats[0].timestamp).unwrap() - Duration::milliseconds(stats[0].latency as i64);
    let endtime = DateTime::parse_from_rfc3339(&stats[stats.len()-1].timestamp).unwrap();
    endtime.signed_duration_since(starttime).num_seconds()
}