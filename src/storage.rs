pub mod influxdb;

use async_trait::async_trait;
use crate::report::stats;

#[async_trait]
pub trait DBWriter {
    async fn write_stats(&mut self, stats: &[stats::Stats]);
}