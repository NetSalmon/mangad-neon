use crate::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use crate::core::entities::dao::crawler::Task;

pub type CanonicalizeResult = Result<PathBuf, Error>;

pub struct CanonicalizeTask {
    pub buffer: Vec<u8>,
    pub base_path: Arc<PathBuf>,
    pub pid: i32,
    pub quality: f32,
    pub repeat: tokio::sync::oneshot::Sender<CanonicalizeResult>,
}

pub struct InnerTask {
    pub task: Task,
    pub id_tx: tokio::sync::oneshot::Sender<i32>,
}