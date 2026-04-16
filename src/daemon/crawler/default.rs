use crate::daemon::crawler::Crawler;
use crate::daemon::models::error::AppError;
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

    async fn handle(&self, subtask: SubTask, client: Arc<Client>) -> Result<Vec<u8>, AppError> {
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

#[cfg(test)]
mod test {
    use super::*;
    #[tokio::test]
    async fn test_crawler() {
        let client = Arc::new(Client::new());
        let crawler = DefaultCrawler {};

        let subtask = SubTask {
            url: "https://192.168.31.194/1231.img".parse().unwrap(),
            headers: Default::default(),
            source_site: Arc::new("all".to_string()),
            index: 0,
            extra: None,
        };

        let res = crawler.handle(subtask, client).await;
        println!("{:#?}", res);
    }
}
