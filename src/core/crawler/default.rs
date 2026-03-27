use std::sync::Arc;
use async_trait::async_trait;
use image::EncodableLayout;
use reqwest::Client;
use crate::core::crawler::Crawler;
use crate::core::entities::dao::crawler::SubTask;
use crate::error::Error;

pub struct DefaultClawer {}

#[async_trait]
impl Crawler for DefaultClawer {
    fn site(&self) -> &str {
        todo!()
    }

    async fn handle(&self, subtask: SubTask, client: Arc<Client>) -> Result<Vec<u8>, Error> {
        let resp = client.get(subtask.url)
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