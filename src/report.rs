use prettytable::{Table, row, cell};
use rayon::prelude::*;

#[derive(Clone, Debug)]
pub struct Stats {
    name: String,
    status: u16,
    time: u128
}

impl Stats {
    pub fn new(name: String, status: u16, time: u128) -> Stats {
        Stats {
            name,
            status,
            time
        }
    }
}

pub fn generate_report(names: Vec<String>, stats: Vec<Stats>, et: u64) {
    let mut table = Table::new();
    table.add_row(row!["Request", "Total Hits", "Hits/s", "Min", "Avg", "Max", "Errors", "Error Rate"]);
     
    let mut total_hits = 0;
    let mut total_errors = 0.0;

    for name in names {
        //HEAVY CLONING HAPPENING HERE - TRY TO FIX
        let filter: Vec<Stats> = stats.clone().into_iter().filter(|s| s.name == name).collect();
        let num = filter.len();
        let min = filter.par_iter().map(|s| s.time).min();
        let max = filter.par_iter().map(|s| s.time).max();
        let sum: u128 = filter.par_iter().map(|s| s.time).sum();
        let sum = sum as usize;
        let avg: usize = sum/num;
        let num_f32 = num as f32;
        let sum_f32 = sum as f32;
        let tput: f32 = (num_f32 * 1000.0) / sum_f32;
        let errors = filter.par_iter().filter(|s| s.status >= 400).count() as f32;
        let error_rate: f32 = errors * 100.0 / num_f32;

        table.add_row(row![&name, &num.to_string(), &tput.to_string(), &min.unwrap().to_string(), 
                            &avg.to_string(), &max.unwrap().to_string(), &errors.to_string(), &error_rate.to_string()]);
        total_hits += num;
        total_errors += errors;
    }

    table.printstd();

    print_summary_table(et, total_hits, total_errors);
}

fn print_summary_table(et: u64, total_hits: usize, total_errors: f32) {
    let mut sum_table = Table::new();
    sum_table.add_row(row!["Total Execution Time (in secs)", "Total Hits", "Hits/s", "Total Errors", "Error Rate"]);

    let et = et as f32;
    let total_hits = total_hits as f32;
    let ttput =  total_hits/et ;
    let err_rate = total_errors * 100.0 / total_hits;

    sum_table.add_row(row![&et.to_string(), &total_hits.to_string(), &ttput.to_string(), &total_errors.to_string(), &err_rate.to_string()]);
    sum_table.printstd();
}