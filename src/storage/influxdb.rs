use async_trait::async_trait;
use chrono::DateTime;
use log::error;
use rustc_hash::FxHashMap as HashMap;

use crate::{converter, model, protocol::http::HttpClient, report::stats, storage};

pub struct InfluxDBWriter {
    pub client: HttpClient,
    request: model::Request,
}

impl InfluxDBWriter {
    pub fn new(db: &model::Database) -> Option<InfluxDBWriter> {
        //check if url is set
        if db.url.is_empty() {
            error!("InfluxDB url or host is not set, not initializing the InfluxDBWriter");
            return None;
        }

        let url= format!("{}/write?db={}&precision=ms", db.url, db.name);

        //Setting headers
        let mut headers = HashMap::default();
        headers.insert(String::from("content-type"), String::from("application/octet-stream"));

         //Setting URL
         if !db.user.is_empty() {
            headers.insert(String::from("authorization"), format!("Basic {}", base64::encode(format!("{}:{}",db.user,db.password))));
        }

        match HttpClient::get_default_async_client() {
            Ok(http_client) =>  Some(InfluxDBWriter {
                client: http_client,
                request: model::Request {
                    id: uuid::Uuid::new_v4(),
                    name: String::from("postToInfluxDB"),
                    url,
                    method: String::from("POST"),
                    body: model::Body {
                        formdata: vec![],
                        urlencoded: HashMap::default(),
                        raw: String::from(""),
                    },
                    headers,
                    extractors: vec![],
                    requires_preprocessing: false
                }
            }),
            Err(err) => {
                error!("Error while initiating new InfluxDB Client {}", err.to_string());
                None
            }
        }
    }

    fn set_body_from_stats(&mut self, stats: &[stats::Stats]) {
        self.request.body.raw = stats.iter()
            .map(|s| {format!("stats,request={} latency={},status={} {}",
                s.name, s.latency, s.status, DateTime::parse_from_rfc3339(&s.timestamp).unwrap().timestamp_millis())})
            .collect::<Vec<String>>().join("\n")
    }
}

#[async_trait]
impl storage::DBWriter for InfluxDBWriter {
    async fn write_stats(&mut self, stats: &[stats::Stats]) {
        //Setting Body
        self.set_body_from_stats(stats);

        let reqwest = converter::convert_request(&self.client, &self.request).await.unwrap();

        match self.client.execute(reqwest).await {
            Ok(_res) => (),
            Err(err) => error!("Error writing to influxdb: {}", err)
        };
    }
}