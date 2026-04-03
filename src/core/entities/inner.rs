use crate::core::entities::dao::crawler::Task;
use crate::error::Error;
use sea_orm::prelude::DateTimeWithTimeZone;
use std::path::PathBuf;
use std::sync::Arc;

pub type CanonicalizeResult = Result<PathBuf, Error>;

pub struct CanonicalizeTask {
    pub buffer: Arc<Vec<u8>>,
    pub base_path: Arc<PathBuf>,
    pub pid: i32,
    pub quality: f32,
    pub repeat: tokio::sync::oneshot::Sender<CanonicalizeResult>,
}

pub struct InnerTask {
    pub task: Task,
    pub id_tx: tokio::sync::oneshot::Sender<i32>,
}

pub enum ExpireTime {
    Short,
    Medium,
    Long,
    Permanent,
    Never,
}

impl ExpireTime {
    pub fn to_duration(&self) -> Option<chrono::Duration> {
        match self {
            ExpireTime::Short => Some(chrono::Duration::minutes(20)),
            ExpireTime::Medium => Some(chrono::Duration::days(7)),
            ExpireTime::Long => Some(chrono::Duration::days(30)),
            ExpireTime::Permanent => Some(chrono::Duration::days(90)),
            ExpireTime::Never => None,
        }
    }

    pub fn get_expire_time(
        &self,
        create_time: DateTimeWithTimeZone,
    ) -> Option<DateTimeWithTimeZone> {
        let Some(duration) = self.to_duration() else {
            return None;
        };

        Some(create_time + duration)
    }
}
