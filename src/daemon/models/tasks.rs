use crate::daemon::models::errors::DaemonError;
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use core::str::FromStr;
use mangad_neon::db::models::Task;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use url::Url;

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

pub type CanonicalizeResult = Result<PathBuf, DaemonError>;

pub fn split(task: &Task) -> Result<Vec<SubTask>, DaemonError> {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubTaskResult {
    pub tid: i32,
    pub index: i32,
    pub status: SubTaskStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum SubTaskStatus {
    Ok,
    Err(String),
}
