use crate::daemon::crawler::Crawler;
use crate::daemon::models::errors::DaemonError;
use crate::daemon::models::tasks::SubTask;
use async_trait::async_trait;
use image::EncodableLayout;
use reqwest::Client;
use std::sync::Arc;

pub struct DefaultCrawler;

#[async_trait]
impl Crawler for DefaultCrawler {
    fn site(&self) -> &str {
        "all"
    }

    async fn handle(&self, subtask: SubTask, client: Arc<Client>) -> Result<Vec<u8>, DaemonError> {
        let resp = client
            .get(subtask.url)
            .headers(subtask.headers)
            .send()
            .await?
            .bytes()
            .await?
            .as_bytes()
            .to_vec();

        Ok(resp)
    }
}
