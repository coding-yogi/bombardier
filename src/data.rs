use std::collections::HashMap;

use csv::{
    Error,
    Reader, 
    ReaderBuilder, 
    Position,
    StringRecord, 
    Trim
};
use log::{info, error};
use serde::de::DeserializeOwned;

use std::io::{Read, Seek};

pub struct DataProvider<R> where R: std::io::Read {
    headers: StringRecord,
    reader: Reader<R>
}

impl<R> DataProvider<R> where R: Read + Seek {
    pub async fn new(rdr: R) -> Self {
        let mut data_provider = DataProvider {
            headers: StringRecord::new(),
            reader: create_reader(rdr).await
        };

        data_provider.headers = data_provider.get_headers().await;
        data_provider
    } 

    pub async fn get_data(&mut self) -> HashMap<String, String> {
        match self.get_record().await {
            Some(record) => self.headers.iter()
                .zip(record.iter())
                .map(|(k,v)| (k.to_owned(), v.to_owned()))
                .collect::<HashMap<String, String>>(),
            None => HashMap::with_capacity(0)
        }
    }

    async fn get_headers(&mut self) -> StringRecord {
        match self.reader.headers() {
            Ok(record) => record.to_owned(),
            Err(err) => {
                error!("Error occurred while reading header row from csv file: {}", err.to_string());
                StringRecord::new()
            }
        }
    }

    async fn get_record(&mut self) -> Option<StringRecord> {
        let mut record = StringRecord::new();
        match self.reader.read_record(&mut record) {
            Ok(record_read) => {
                if record_read {
                    return Some(record);
                } else {
                    info!("End of file reached for data file, reseting position");
                    let _ = self.reader.seek(Position::new());
                    let _ = self.reader.read_record(&mut record); //Ignoring header row
                    let _ = self.reader.read_record(&mut record);
                    return Some(record);
                }
            }, 
            Err(err) => {
                error!("Error occurred while reading record {}", err.to_string());
                return None;
            }
        }
    }

    pub async fn get_records_as<T>(&mut self) -> Result<Vec<T>, Error> 
    where T: DeserializeOwned {       
        let _ = self.reader.seek(Position::new());
        let record_stream = self.reader.records();

        let headers = self.headers.to_owned();

        let vec = record_stream.map(|r| {
            let sr = match r {
                Ok(sr) => sr,
                Err(err) => {
                    error!("Error occurred while reading record stream {}", err.into());
                    Err(err)
                }
            };
            let s: T = sr.deserialize(Some(&headers)).unwrap();
            
        }).collect::<Vec<T>>();

        Ok(vec)
    }
}

async fn create_reader<R: std::io::Read>(rdr: R) -> Reader<R> {
    ReaderBuilder::new()
        .has_headers(true)
        .trim(Trim::All)
        .from_reader(rdr)
}