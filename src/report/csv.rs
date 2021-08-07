use log::warn;
use std::{fs, io::Write};

use crate::{file, report::stats};

pub struct CSVWriter {
    report_file: fs::File
}

impl CSVWriter {
    pub fn new(report_file: &str) -> Result<CSVWriter, std::io::Error> {
        let file = file::create_file(report_file)?;
        let mut csv_writer = CSVWriter {
            report_file: file
        };

        //write header row
        csv_writer.report_file.write(&format!("timestamp, status, latency, name\n").as_bytes())?;
        Ok(csv_writer)
    }
}

impl stats::StatsWriter for CSVWriter {
    fn write_stats(&mut self, stats: &Vec<stats::Stats>) {
        for stat in stats {
            match self.report_file.write(stat.to_string().as_bytes()) {
                Err(err) => warn!("Unable to write stat {} to file due to error {}", stat, err),
                Ok(_) => (),
            }
        }
    }
}

