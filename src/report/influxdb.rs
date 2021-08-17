use chrono::DateTime;
use log::error;
use reqwest::Client;
use serde_yaml::{Mapping, Value};

use crate::{
    cmd,
    model::scenarios,
    protocol::http,
    report::stats,
};

pub struct InfluxDBWriter {
    pub client: Client,
    request: scenarios::Request,
}

impl InfluxDBWriter {
    pub fn new(influxdb: &cmd::InfluxDB, client: Client) -> Option<InfluxDBWriter> {

        //check if url is set
        if influxdb.url == "" {
            return None;
        }

        //Setting URL
        let mut url = format!("{}/write?db={}&precision=ms", influxdb.url, influxdb.dbname);
        if influxdb.username != "" {
            url =  format!("{}&u={}&p={}", url, influxdb.username, influxdb.password);
        }

        //Setting headers
        let mut headers = serde_yaml::Mapping::with_capacity(1);
        headers.insert(Value::from("content-type"), Value::from("application/octet-stream"));

        Some(InfluxDBWriter {
            client: client,
            request: scenarios::Request {
                name: String::from("postToInfluxDB"),
                url: url,
                method: String::from("POST"),
                body: scenarios::Body {
                    formdata: Mapping::with_capacity(0),
                    urlencoded: Mapping::with_capacity(0),
                    raw: String::from(""),
                },
                headers: headers,
                extractor: scenarios::Extractor {
                    gjson_path: Mapping::with_capacity(0),
                    xpath: Mapping::with_capacity(0),
                    regex: Mapping::with_capacity(0)
                }
            }
        })
    }

    pub async fn write_stats(&mut self, stats: &Vec<stats::Stats>) {
        //Setting Body
        self.set_body_from_stats(&stats);

        match http::execute(&self.client, &self.request).await {
            Ok(_res) => (),
            Err(err) => error!("Error writing to influxdb: {}", err)
        };
    }

    fn set_body_from_stats(&mut self, stats: &Vec<stats::Stats>) {
        self.request.body.raw = stats.iter()
            .map(|s| {format!("stats,request={} latency={},status={} {}",
                s.name, s.latency, s.status, DateTime::parse_from_rfc3339(&s.timestamp).unwrap().timestamp_millis())})
            .collect::<Vec<String>>().join("\n")
    }
}