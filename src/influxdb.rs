
use crate::cmd;
use crate::report;

use chrono::DateTime;
use log::error;
use reqwest::blocking::Client;

pub struct InfluxDBClient {
    pub influxdb: cmd::InfluxDB,
    pub client: Client
}

impl InfluxDBClient {
    pub fn write_stats(&mut self, stats: Vec<report::Stats>) {
        let mut body = String::from("");

        let mut url = format!("{}/write?db={}&precision=ms", self.influxdb.url, self.influxdb.dbname);
        if self.influxdb.username != "" {
            url =  format!("{}&u={}&p={}", url, self.influxdb.username, self.influxdb.password);
        }

        let request_builder = self.client.post(&url).header("content-type","application/octet-stream");

        for stat in stats {
            body = format!("{}stats,request={} latency={},status={} {}\n",
            body, stat.name, stat.latency, stat.status, DateTime::parse_from_rfc3339(&stat.timestamp).unwrap().timestamp_millis());
        }

        match request_builder.body(body).send() {
            Ok(_res) => (),
            Err(err) => error!("Error writing to influxdb: {}", err)
        };
    }
}