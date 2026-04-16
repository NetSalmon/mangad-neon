use core::str::FromStr;
use std::path::PathBuf;
use std::sync::Arc;
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use mangad_neon::core::entities::dao::crawler::Task;
use url::Url;
use crate::models::error::AppError;

pub struct CanonicalizeTask {
    pub buffer: Arc<Vec<u8>>,
    pub format: Option<String>,
    pub base_path: Arc<PathBuf>,
    pub pid: i32,
    pub quality: f32,
    pub repeat: tokio::sync::oneshot::Sender<CanonicalizeResult>,
}

pub struct ReturningTask {
    pub task: Task,
    pub tid_tx: tokio::sync::oneshot::Sender<i32>,
}

pub type CanonicalizeResult = Result<PathBuf, AppError>;

pub fn split(task: &Task) -> Result<Vec<SubTask>, AppError> {
    let mut subtasks = vec![];

    let mut headers = HeaderMap::new();
    for (k, v) in task.headers.iter() {
        headers.insert(HeaderName::from_str(k)?, HeaderValue::from_str(v)?);
    }

    let source_site = Arc::new(task.source_site.clone());
    let extra = if let Some(data) = task.extra.clone() {
        Some(Arc::new(data))
    } else {
        None
    };

    for (index, image) in task.images.iter().enumerate() {
        let subtask = SubTask {
            url: image.parse()?,
            headers: headers.clone(),
            source_site: source_site.clone(),
            index: index as i32,
            extra: extra.clone(),
        };

        subtasks.push(subtask);
    }

    Ok(subtasks)
}

#[derive(Clone)]
pub struct SubTask {
    pub url: Url,
    pub headers: HeaderMap,
    pub source_site: Arc<String>,
    pub index: i32,
    pub extra: Option<Arc<serde_json::Value>>,
}