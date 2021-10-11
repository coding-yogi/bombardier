pub mod csv;
pub mod stats;

use chrono::{DateTime, Duration};
use prettytable::{Table, row, cell};
use rayon::prelude::*;
use rustc_hash::FxHashSet as HashSet;
use log::error;

use crate::report::stats::Stats;
use crate::data::{self, DataProvider};

pub async fn display(report_file: &str) -> Result<(), Box<dyn std::error::Error>> {

    let mut data_provider = get_data_provider(report_file).await?;

    let stats= get_stats(&mut data_provider).await?;
    let names = get_request_name_set(&stats);

    let mut table = Table::new();
    table.add_row(row![FY => "Request", "Total Hits", "Hits/s", "Min", "Avg", "Max", "90%", "95%", "99%", "Errors", "Error Rate"]);
        
    let mut total_hits = 0;
    let mut total_errors = 0;

    let et = get_execution_time(&stats);

    for name in names {
        let name_filter: Vec<&Stats> = filter_stats_by_name(&stats, name);
        let num = name_filter.len();

        let latencies: Vec<u32> = get_sorted_latencies(&name_filter);

        let (min, max) = (latencies[0], latencies[num-1]);
        let (pc_90, pc_95, pc_99) = get_all_percentiles(&latencies);

        let avg= sum_of_latencies(&latencies) / num;
        let tput= num as f32 / et as f32;
        let errors = get_error_count(&name_filter);
        let error_rate= errors as f32 * 100.0 / num as f32;

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

async fn get_data_provider(report_file: &str) -> Result<DataProvider, Box<dyn std::error::Error>> {
    match data::DataProvider::new(report_file).await {
        Some(data_provider) => Ok(data_provider),
        None => {
            error!("Unable to initialize data provider for {}", report_file);
            return Err("Unable to initialize data provider".into())
        }
    }
}

async fn get_stats(data_provider: &mut DataProvider) -> Result<Vec<Stats>, Box<dyn std::error::Error>> {
    let mut stats: Vec<Stats> = data_provider.get_records_as().await.unwrap();
    stats.sort_by_key(|k| k.timestamp.clone()); //required for distributed execution
    Ok(stats)
}

fn get_request_name_set(stats: &[Stats]) -> HashSet<&str> {
    stats.iter()
        .map(|s| s.name.as_str())
        .collect()
}

fn filter_stats_by_name<'a>(stats: &'a [Stats], name: &str) -> Vec<&'a Stats> {
    stats.par_iter()
        .filter(|s| s.name == name)
        .collect()
}

fn get_sorted_latencies(stats: &[&Stats]) -> Vec<u32> {
    let mut latencies = stats.par_iter()
    .map(|s| s.latency)
    .collect::<Vec<u32>>();

    latencies.par_sort_unstable();
    latencies
}

fn get_error_count(stats: &[&Stats]) -> usize {
    stats.par_iter()
        .filter(|s| s.status >= 400)
        .count()
}

fn sum_of_latencies(latencies: &[u32]) -> usize {
    latencies.par_iter().sum::<u32>() as usize
}

fn get_percentile(sorted_vector: &[u32], p: usize) -> u32 {
    let len = sorted_vector.len();

    if len == 0 {
        return 0
    }

    match p*len/100 {
        0 => sorted_vector[0],
        _ => sorted_vector[(p*len/100)-1]
    }
}

fn get_all_percentiles(times: &[u32]) -> (u32, u32, u32) {
    (get_percentile(times, 90), get_percentile(times, 95), get_percentile(times, 99))
}

fn print_summary_table(et: i64, total_hits: usize, total_errors: usize) {
    let mut sum_table = Table::new();
    sum_table.add_row(row![FG => "Total Execution Time (in secs)", "Total Hits", "Hits/s", "Total Errors", "Error Rate"]);

    let ttput =  total_hits as f32 / et as f32;
    let err_rate = total_errors as f32 * 100.0 / total_hits as f32;

    sum_table.add_row(row![&et.to_string(), &total_hits.to_string(), &ttput.to_string(), &total_errors.to_string(), &err_rate.to_string()]);
    sum_table.printstd();
}

fn get_execution_time(stats: &[Stats]) -> i64 {
    if stats.len() == 0 {
        return 0
    }

    let format = "%Y-%m-%d %H:%M:%S%.6f %z";
    let starttime = DateTime::parse_from_str(&stats[0].timestamp, format).unwrap() - Duration::milliseconds(stats[0].latency as i64);
    let endtime = DateTime::parse_from_str(&stats[stats.len()-1].timestamp, format).unwrap();
    endtime.signed_duration_since(starttime).num_seconds()
}

#[test]
fn test_get_percentile() {
    let times = &[200, 203, 210, 256, 315]; //must be sorted slice
    assert_eq!(get_percentile(times, 10), 200);
    assert_eq!(get_percentile(times, 59), 203);
    assert_eq!(get_percentile(times, 60), 210);
    assert_eq!(get_percentile(times, 90), 256);
    assert_eq!(get_percentile(&[], 51), 0);
}

#[test]
fn test_get_all_percentile() {
    let times = &[1,2,3,4,5,6,7,8,9,10]; //must be sorted slice
    assert_eq!(get_all_percentiles(times), (9,9,9));
}

#[test]
fn test_get_execution_time() {
    //Latency of 1st element is subtracted from its timestamp to get start time 
    //so if 1st element takes 1 sec, the execution time should be 2+1
    let stats1 = Stats::new("name1", 200, 1000, 1); 
    std::thread::sleep(std::time::Duration::from_secs(2));
    let stats2 = Stats::new("name2", 200, 150, 1);
    assert_eq!(get_execution_time(&[stats1, stats2]), 3);

    //Check negative 
    let stats1 = Stats::new("name1", 200, 0, 1);
    std::thread::sleep(std::time::Duration::from_secs(1));
    let stats2 = Stats::new("name2", 200, 0, 1);
    assert_eq!(get_execution_time(&[stats2, stats1]), -1);

    //check empty
    assert_eq!(get_execution_time(&[]), 0);
}

#[test]
fn test_get_request_name_set() {
    let stats = vec![Stats::new("name1", 0, 0, 0), Stats::new("name1", 0, 0, 0), Stats::new("name2", 0, 0, 0)];
    let names = get_request_name_set(&stats);
    assert_eq!(names.len(), 2);

    let names = get_request_name_set(&[]);
    assert_eq!(names.len(), 0);
}

#[test]
fn test_filter_stats_by_name() {
    let stats = vec![Stats::new("name1", 0, 0, 0), Stats::new("name1", 0, 0, 0), Stats::new("name2", 0, 0, 0)];
    assert_eq!(filter_stats_by_name(&stats, "name1").len(), 2);
    assert_eq!(filter_stats_by_name(&stats, "name2").len(), 1);
    assert_eq!(filter_stats_by_name(&stats, "name3").len(), 0);
    assert_eq!(filter_stats_by_name(&[], "name2").len(), 0);
}

#[test]
fn test_get_sorted_latencies() {
    let stats1 = Stats::new("name1", 0, 250, 0);
    let stats2 = Stats::new("name1", 0, 100, 0);
    let stats3 = Stats::new("name1", 0, 300, 0);
    let stats4 = Stats::new("name1", 0, 50, 0);
    let stats = vec![&stats1, &stats2, &stats3, &stats4];

    assert_eq!(get_sorted_latencies(&stats), vec![stats4.latency,stats2.latency,stats1.latency,stats3.latency]);
}

#[test]
fn test_get_error_count() {
    let stats1 = Stats::new("name1", 200, 250, 0);
    let stats2 = Stats::new("name1", 399, 100, 0);
    let stats3 = Stats::new("name1", 400, 300, 0);
    let stats4 = Stats::new("name1", 504, 50, 0);
    let stats = vec![&stats1, &stats2, &stats3, &stats4];

    assert_eq!(get_error_count(&stats), 2);
}