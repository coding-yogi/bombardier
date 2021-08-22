use csv_async::{
    AsyncReader, 
    AsyncReaderBuilder, 
    Error, 
    StringRecord, 
    Trim
};
use futures::StreamExt;
use log::warn;
use serde::de::DeserializeOwned;
use tokio::{fs, io::{AsyncWriteExt, AsyncRead}};

use std::collections::HashMap;
use std::fmt::Display;

use crate::file;

pub struct CSVReader;

impl CSVReader {

    fn get_reader<R>(&self, rdr: R, has_headers: bool) -> AsyncReader<R> 
    where R: AsyncRead + Unpin + Send + Sync {
        AsyncReaderBuilder::new()
                .has_headers(has_headers)
                .trim(Trim::All)
                .create_reader(rdr)
    }

    pub async fn get_records<R>(&self, rdr: R) -> Result<Vec<HashMap<String, String>>, Error> 
    where R: AsyncRead + Unpin + Send + Sync {

        let mut reader = self.get_reader(rdr, false);
        let mut record_stream = reader.records();

        let headers= match record_stream.next().await {
            Some(item) => {
                match item {
                    Ok(item) => item.iter()
                    .map(|s| s.to_owned())
                    .collect(),
                    Err(err) => return Err(err)
                }
            },
            None => Vec::new()
        };

        let vec_data_map = record_stream.map(|record| {
            headers.iter()
                .zip(record.unwrap().iter())
                .map(|(k,v)| (k.to_owned(), v.to_owned()))
                .collect::<HashMap<String, String>>()
        }).collect::<Vec<HashMap<String, String>>>().await;
        
        Ok(vec_data_map)
    }

    pub async fn get_records_as<T, R>(&self, rdr: R, headers: &StringRecord) -> Result<Vec<T>, Error>
    where R: AsyncRead + Unpin + Send + Sync, T: DeserializeOwned {

        let mut reader = self.get_reader(rdr, true);
        let record_stream = reader.records();

        let vec = record_stream.map(|r| {
            let sr = r.unwrap();
            let s: T = sr.deserialize(Some(&headers)).unwrap();
            s
        }).collect::<Vec<T>>().await;

        Ok(vec)
    }
}

pub fn string_record_from_vec(vec: &[&str]) -> StringRecord {
    csv_async::StringRecord::from(vec)
}

pub struct CSVWriter {
    report_file: fs::File
}

impl CSVWriter {
    pub async fn new(report_file: &str) -> Result<CSVWriter, std::io::Error> {
        let file = file::create_file(report_file).await?;
        let mut csv_writer = CSVWriter {
            report_file: file
        };

        //write header row
        csv_writer.report_file.write_all(&format!("timestamp, status, latency, name\n").as_bytes()).await?;
        Ok(csv_writer)
    }

    pub async fn write<T: Display>(&mut self, stats: &[T]) {
        for stat in stats {
            match self.report_file.write_all(stat.to_string().as_bytes()).await {
                Err(err) => warn!("Unable to write stat {} to file due to error {}", stat, err),
                Ok(_) => (),
            }
        }
    }
}