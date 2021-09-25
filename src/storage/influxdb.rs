use async_trait::async_trait;
use chrono::DateTime;
use log::error;
use reqwest::Client;
use serde_yaml::{Mapping, Value};

use crate::{
    cmd,
    model,
    protocol::http,
    report::stats,
    storage
};

pub struct InfluxDBWriter {
    pub client: Client,
    request: model::Request,
}

impl InfluxDBWriter {
    pub fn new(influxdb: &cmd::Database) -> Option<InfluxDBWriter> {
        //check if url is set
        if influxdb.url == "" {
            error!("InfluxDB url is not set, not initializing the InfluxDBWriter");
            return None;
        }

        //Setting URL
        let mut str_url = format!("{}/write?db={}&precision=ms", influxdb.url, influxdb.name);
        if influxdb.user != "" {
            str_url =  format!("{}&u={}&p={}", str_url, influxdb.user, influxdb.password);
        }

        let url = match url::Url::parse(&str_url) {
            Ok(url) => url,
            Err(err) => {
                error!("Error occurred while parsing influx DB url {}", err);
                return None;
            }
        };

        //Setting headers
        let mut headers = serde_yaml::Mapping::with_capacity(1);
        headers.insert(Value::from("content-type"), Value::from("application/octet-stream"));

        Some(InfluxDBWriter {
            client: http::get_default_sync_client(),
            request: model::Request {
                name: String::from("postToInfluxDB"),
                url: url,
                method: String::from("POST"),
                body: model::Body {
                    formdata: Mapping::with_capacity(0),
                    urlencoded: Mapping::with_capacity(0),
                    raw: String::from(""),
                },
                headers: headers,
                extractors: vec![]
            }
        })
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
        self.set_body_from_stats(&stats);

        match http::execute(&self.client, &self.request).await {
            Ok(_res) => (),
            Err(err) => error!("Error writing to influxdb: {}", err)
        };
    }
}