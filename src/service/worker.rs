use crate::canonicalize::Canonicalization;
use crate::crawler::Dispatch;
use crate::searching;
use crate::searching::sync;
use crate::service::handlers::basic::SpawnStatus;
use crate::thumbnail::{Thumbnail, ThumbnailTask};
use mangad_neon::CHANNEL_SIZE;
use mangad_neon::core::config::Config;
use mangad_neon::core::dao::SubTaskResult;
use crate::models::tasks::ReturningTask;
use mangad_neon::core::orm::tasks;
use mangad_neon::core::repository::{IntoDatabaseUrl, Repository};
use meilisearch_sdk::indexes::Index;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, watch};
use crate::models::error::AppError;

pub struct Worker {
    index: Arc<Index>,
    repo: Arc<Repository>,
    thumbnail: Thumbnail,
    dispatch: Dispatch,
    canonicalization: Canonicalization,

    thumbnail_tx: watch::Sender<SpawnStatus>,
    sync_tx: watch::Sender<SpawnStatus>,
    dispatch_tx: watch::Sender<SpawnStatus>,
    canonicalization_tx: watch::Sender<SpawnStatus>,
}

pub struct WorkerHandler {
    pub repo: Arc<Repository>,
    pub index: Arc<Index>,
    pub sub_task_tx: broadcast::Sender<SubTaskResult>,
    pub thumbnail_tx: mpsc::Sender<ThumbnailTask>,
    pub task_tx: broadcast::Sender<tasks::Model>,
    pub dispatch_tx: mpsc::Sender<ReturningTask>,
    pub watch: Watch,

    _task_rx: broadcast::Receiver<tasks::Model>,
    _sub_task_rx: broadcast::Receiver<SubTaskResult>,
}

pub struct Watch {
    pub thumbnail: watch::Receiver<SpawnStatus>,
    pub sync: watch::Receiver<SpawnStatus>,
    pub dispatch: watch::Receiver<SpawnStatus>,
    pub canonicalization: watch::Receiver<SpawnStatus>,
}

impl Worker {
    pub async fn new(config: Arc<Config>) -> Result<(Self, WorkerHandler), AppError> {
        let (thumbnail, thumbnail_tx) = Thumbnail::new(config.clone());
        let (canonicalization, canonical_tx) = Canonicalization::new(config.clone());

        let (sub_task_tx, sub_task_rx) = broadcast::channel::<SubTaskResult>(CHANNEL_SIZE);
        let (task_tx, task_rx) = broadcast::channel::<tasks::Model>(CHANNEL_SIZE);

        let repo = Arc::new(Repository::new(&config.to_database_url()).await?);
        let index = Arc::new(searching::index(config.clone()).await?);

        let (thumbnail_watch_tx, thumbnail_watch_rx) =
            watch::channel::<SpawnStatus>(Default::default());
        let (sync_watch_tx, sync_watch_rx) = watch::channel::<SpawnStatus>(Default::default());
        let (dispatch_watch_tx, dispatch_watch_rx) =
            watch::channel::<SpawnStatus>(Default::default());
        let (canonicalization_tx, canonicalization_rx) =
            watch::channel::<SpawnStatus>(Default::default());

        let watch = Watch {
            thumbnail: thumbnail_watch_rx,
            sync: sync_watch_rx,
            dispatch: dispatch_watch_rx,
            canonicalization: canonicalization_rx,
        };

        let (dispatch, dispatch_tx) = Dispatch::new(
            config.clone(),
            repo.clone(),
            task_tx.clone(),
            sub_task_tx.clone(),
            thumbnail_tx.clone(),
            canonical_tx.clone(),
        );

        let data = Self {
            thumbnail,
            index: index.clone(),
            dispatch,
            repo: repo.clone(),
            canonicalization,

            thumbnail_tx: thumbnail_watch_tx,
            sync_tx: sync_watch_tx,
            dispatch_tx: dispatch_watch_tx,
            canonicalization_tx,
        };

        let handler = WorkerHandler {
            repo,
            index,
            dispatch_tx,
            thumbnail_tx,
            task_tx,
            sub_task_tx,
            watch,

            _task_rx: task_rx,
            _sub_task_rx: sub_task_rx,
        };

        Ok((data, handler))
    }

    pub async fn spawn(mut self) {
        tokio::spawn(async move {
            let _ = self.thumbnail_tx.send(SpawnStatus::Running);
            if let Err(err) = self.thumbnail.run().await {
                let _ = self.thumbnail_tx.send(SpawnStatus::Error {
                    message: err.to_string(),
                });
                tracing::error!("Thumbnail encode error: {}", err);
            }
        });

        tokio::spawn(async move {
            let _ = self.canonicalization_tx.send(SpawnStatus::Running);
            if self.canonicalization.run().await == () {
                let _ = self.canonicalization_tx.send(SpawnStatus::Error {
                    message: "".to_string(),
                });
                tracing::error!("canonicalization stop");
            }
        });

        tokio::spawn(async move {
            let _ = self.dispatch_tx.send(SpawnStatus::Running);
            if let Err(err) = self.dispatch.run().await {
                let _ = self.dispatch_tx.send(SpawnStatus::Error {
                    message: err.to_string(),
                });
                tracing::error!("Crawler dispatch error: {:?}", err);
            }
        });

        tokio::spawn(async move {
            let _ = self.sync_tx.send(SpawnStatus::Running);
            if let Err(err) = sync(self.repo.clone(), self.index).await {
                let _ = self.sync_tx.send(SpawnStatus::Error {
                    message: err.to_string(),
                });
                tracing::error!("Search sync error: {:?}", err);
            }
        });
    }
}
