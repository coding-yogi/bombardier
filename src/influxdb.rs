
use crate::cmd;
use crate::report;

use chrono::DateTime;
use reqwest::{Client, RequestBuilder};

pub fn build_request(client: &Client, influxdb: &cmd::InfluxDB) -> RequestBuilder {
    let mut url = format!("{}/write?db={}&precision=ms", influxdb.url, influxdb.dbname);
    if influxdb.username != "" {
        url =  format!("{}&u={}&p={}", url, influxdb.username, influxdb.password);
    }

    client.post(&url).header("content-type","application/octet-stream")
}

pub async fn write_stats(request: RequestBuilder, stats: Vec<report::Stats>) {
    let mut body = String::from("");
    for stat in stats {
        body = format!("{}stats,request={} latency={},status={} {}\n",
        body, stat.name, stat.latency, stat.status, DateTime::parse_from_rfc3339(&stat.timestamp).unwrap().timestamp_millis());
    }

    request.body(body).send().await;
}