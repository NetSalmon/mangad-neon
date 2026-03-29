use std::path::PathBuf;
use crate::error::Error;

pub type CanonicalizeResult = Result<PathBuf, Error>;

pub struct CanonicalizeTask {
    pub buffer: Vec<u8>,
    pub base_path: PathBuf,
    pub tid: i32,
    pub pid: i32,
    pub quality: f32,
    pub repeat: tokio::sync::oneshot::Sender<CanonicalizeResult>,
}
