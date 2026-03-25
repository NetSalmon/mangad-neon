use std::collections::HashMap;
use url::Url;

pub struct Task {
    pub images: Vec<String>,
    pub headers: HashMap<String, String>,
    pub source_site: String,
    pub source_id: String,
}

pub struct SubTask {
    pub url: String,
    pub headers: HashMap<String, String>,
    pub source_site: String,
    pub source_id: String,
    pub index: usize,
}

impl Task {
    pub fn spilt(&self) -> Vec<SubTask> {
        self.images
            .iter()
            .enumerate()
            .map(|(index, url)| {
                SubTask {
                    url: url.clone(),
                    headers: self.headers.clone(),
                    source_id: self.source_id.clone(),
                    source_site: self.source_site.clone(),
                    index
                }
            })
            .collect()
    }
}