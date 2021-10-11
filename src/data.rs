use csv_async::{
    AsyncReader, 
    AsyncReaderBuilder, 
    Error, 
    StringRecord, 
    Trim
};

use futures::StreamExt;
use log::{info, error};
use serde::de::DeserializeOwned;
use rustc_hash::FxHashMap as HashMap;
use tokio::{
    io::AsyncRead,
    fs::File
};

pub struct DataProvider  {
    file_path: String,
    headers: StringRecord,
    reader: AsyncReader<File>
}

impl DataProvider {
    pub async fn new(file_path: &str) -> Option<Self> {
        if file_path.trim().is_empty() {
            return None
        }

        let file = match File::open(file_path).await {
            Ok(file) => file,
            Err(err) => {
                error!("Error while reading data file {}", err);
                return None
            }
        };

        let mut data_provider = DataProvider {
            file_path: file_path.to_string(),
            headers: StringRecord::new(),
            reader: create_reader(file).await
        };

        data_provider.headers = data_provider.get_headers().await;
        Some(data_provider)
    } 

    pub async fn get_data(&mut self) -> HashMap<String, String> {
        match self.get_record().await {
            Some(record) => self.headers.iter()
                .zip(record.iter())
                .map(|(k,v)| (k.to_owned(), v.to_owned()))
                .collect::<HashMap<String, String>>(),
            None => HashMap::default()
        }
    }

    async fn get_headers(&mut self) -> StringRecord {
        match self.reader.headers().await {
            Ok(record) => record.to_owned(),
            Err(err) => {
                error!("Error occurred while reading header row from csv file: {}", err.to_string());
                StringRecord::new()
            }
        }
    }

    async fn get_record(&mut self) -> Option<StringRecord> {
        let mut record = StringRecord::new();
        match self.reader.read_record(&mut record).await {
            Ok(record_read) => {
                if !record_read {
                    info!("End of file reached for data file, reseting position");
                    let file = File::open(&self.file_path).await.unwrap();
                    self.reader = create_reader(file).await;
                    let _record_read = self.reader.read_record(&mut record).await;
                } 
                    
                Some(record)
            }, 
            Err(err) => {
                error!("Error occurred while reading record {}", err.to_string());
                None
            }
        }
    }

    pub async fn get_records_as<T>(&mut self) -> Result<Vec<T>, Error> 
    where T: DeserializeOwned {       
        let file = File::open(&self.file_path).await.unwrap();
        self.reader = create_reader(file).await;
        let record_stream = self.reader.records();
        let headers = self.headers.to_owned();

        let vec = record_stream.map(|r| {
            let sr = r.unwrap();
            let s: T = sr.deserialize(Some(&headers)).unwrap();
            s
        }).collect::<Vec<T>>().await;

        Ok(vec)
    }
}

async fn create_reader<R: AsyncRead + Unpin + Send + Sync>(rdr: R) -> AsyncReader<R> {
    AsyncReaderBuilder::new()
        .has_headers(true)
        .trim(Trim::All)
        .create_reader(rdr)
}


#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{Error, Write},
        path::PathBuf
    };
    use serde::Deserialize;
    use tempdir::TempDir;

    use super::DataProvider;

    #[derive(Deserialize)]
    struct Data {
        header1: String,
        header2: String,
    }

    fn setup_temp_csv_file(dir_name: &str, file_name: &str) -> Result<(TempDir, PathBuf), Error>{
        let dir = TempDir::new(dir_name)?;
        let file_path = dir.path().join(file_name);
        let file_path_clone = file_path.clone();
        let mut file = File::create(file_path_clone)?;
        file.write_all(b"header1, header2, header3\nvalue11, value12, value13\nvalue21, value22, value23")?;
        file.sync_all()?;
        Ok((dir, file_path))
    }

    #[tokio::test]
    async fn test_get_data() {
        let (_dir, file_path) = setup_temp_csv_file("test_get_headers", "test.csv").unwrap();
        let mut data_provider = DataProvider::new(file_path.to_str().unwrap()).await.unwrap();
       
        let data = data_provider.get_data().await;
        assert_eq!(data.get("header1").unwrap(), "value11");

        let data = data_provider.get_data().await;
        assert_eq!(data.get("header2").unwrap(), "value22");

        //data row should reset to 1 row after end of file
        let data = data_provider.get_data().await; 
        assert_eq!(data.get("header3").unwrap(), "value13");
    }

    #[tokio::test]
    async fn test_invalid_file_path() {
        let data_provider = DataProvider::new("some_file.csv").await;
        assert!(data_provider.is_none());
    }

    #[tokio::test]
    async fn test_get_record_as() {
        let (_dir, file_path) = setup_temp_csv_file("test_get_record_as", "test.csv").unwrap();
        let mut data_provider = DataProvider::new(file_path.to_str().unwrap()).await.unwrap();
       
        let data = data_provider.get_records_as::<Data>().await.unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0].header1, "value11");
        assert_eq!(data[1].header2, "value22");
    }
}