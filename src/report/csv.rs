use log::{info,warn};
use tokio::{fs, io::AsyncWriteExt};

use std::fmt::Display;

pub struct CSVWriter {
    report_file: fs::File
}

impl CSVWriter {
    pub async fn new(report_file: &str) -> Result<CSVWriter, std::io::Error> {
        info!("Initiating CSVWriter");
        let file = fs::File::create(report_file).await?;
        let mut csv_writer = CSVWriter {
            report_file: file
        };

        //write header row
        csv_writer.report_file.write_all("timestamp, status, latency, name\n".as_bytes()).await?;
        Ok(csv_writer)
    }

    pub async fn write<T: Display>(&mut self, stats: &[T]) {
        for stat in stats {
            if let Err(err) =  self.report_file.write_all(stat.to_string().as_bytes()).await {
                warn!("Unable to write stat {} to file due to error {}", stat, err)
            }
        }
    }
}