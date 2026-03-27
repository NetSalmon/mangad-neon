use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use url::Url;
use crate::error::Error;

pub struct Task {
    pub id: i32,
    pub images: Vec<String>,
    pub headers: HashMap<String, String>,
    pub source_site: String,
    pub source_id: String,
}

#[derive(Clone)]
pub struct SubTask {
    pub url: Url,
    pub headers: HeaderMap,
    pub source_site: Arc<String>,
    pub source_id: Arc<String>,
    pub index: usize,
}

impl Task {
    pub fn spilt(&self) -> Result<Vec<SubTask>, Error> {
        let mut subtasks = vec![];

        let mut headers = HeaderMap::new();
        for (k, v) in self.headers.iter() {
            headers.insert(HeaderName::from_str(k)?, HeaderValue::from_str(v)?);
        }

        let source_site = Arc::new(self.source_site.clone());
        let source_id = Arc::new(self.source_id.clone());

        for (index, image) in self.images.iter().enumerate() {
            let subtask = SubTask {
                url: image.parse()?,
                headers: headers.clone(),
                source_site: source_site.clone(),
                source_id: source_id.clone(),
                index,
            };

            subtasks.push(subtask);
        }

        Ok(subtasks)
    }
}