use crate::daemon::crawler::Crawler;
use crate::daemon::models::errors::DaemonError;
use crate::daemon::models::tasks::SubTask;
use async_trait::async_trait;
use md5::{Digest, Md5};
use reqwest::Client;
use std::sync::Arc;

pub struct JmComicCrawler;

#[async_trait]
impl Crawler for JmComicCrawler {
    fn site(&self) -> &str {
        "jmcomic"
    }

    async fn handle(&self, subtask: SubTask, client: Arc<Client>) -> Result<Vec<u8>, DaemonError> {
        let id = subtask
            .extra
            .ok_or(DaemonError::CustomError("no extra data".to_string()))?
            .get("source_id")
            .ok_or(DaemonError::CustomError("no id".to_string()))?
            .as_str()
            .ok_or(DaemonError::CustomError("id empty".to_string()))?
            .parse::<u64>()?;

        let layers = {
            let n = format!("{}{:0>5}", id, subtask.index);
            let mut hasher = Md5::new();
            hasher.update(n.as_bytes());
            let result = hasher.finalize();
            let last_byte = result.last().unwrap();
            let last_char_code = format!("{:x}", last_byte).chars().last().unwrap() as u32;

            (match id {
                268850..=421925 => last_char_code % 10,
                421926.. => last_char_code % 8,
                _ => 4,
            }) * 2
                + 2
        };

        let resp = client
            .get(subtask.url)
            .headers(subtask.headers)
            .send()
            .await?
            .bytes()
            .await?;

        let img = image::load_from_memory(&resp)?;

        drop(resp);

        let (width, height) = (img.width(), img.height());

        let mut canvas = image::DynamicImage::new(width, height, img.color());
        let avg_height = height / layers;
        let remainder = height % layers;

        for i in 0..layers {
            let src_y = i * avg_height;
            let target_y = if i == layers - 1 {
                0
            } else {
                height - (avg_height * (i + 1)) - remainder
            };
            let current_part_height = if i == layers - 1 {
                avg_height + remainder
            } else {
                avg_height
            };

            let part = img.crop_imm(0, src_y, width, current_part_height);
            image::imageops::replace(&mut canvas, &part, 0, target_y as i64);
        }

        Ok(canvas.as_bytes().to_vec())
    }
}
