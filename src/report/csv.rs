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
        csv_writer.report_file.write_all("timestamp, thread_count, status, latency, name\n".as_bytes()).await?;
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

#[cfg(test)]
mod tests {
    use tempdir::TempDir;
    use tokio::fs::File;
    use tokio::io::AsyncReadExt; 
    use std::str;

    use super::CSVWriter;
    use crate::report::stats::Stats;
    
    #[tokio::test]
    async fn test_write_to_csv() {
        let dir = TempDir::new("test_write_to_csv").unwrap();
        let file_path = dir.path().join("test.csv");

        let mut csv_writer = CSVWriter::new(file_path.to_str().unwrap()).await.unwrap();
        let stats = vec![Stats::new("test", 200, 200, 1)];

        csv_writer.write(&stats).await;

        let mut file = File::open(file_path).await.unwrap();
        let mut contents = vec![];
        file.read_to_end(&mut contents).await.unwrap();

        let contents = str::from_utf8(&contents).unwrap().split('\n').collect::<Vec<_>>();
        assert!(contents[0].contains("timestamp, thread_count, status, latency, name"));
        assert!(contents[1].contains("1, 200, 200, test"));
    }
} 