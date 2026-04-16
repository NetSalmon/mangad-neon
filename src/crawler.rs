use crate::crawler::jmcomic::JmComicCrawler;
use crate::thumbnail::{TaskType, ThumbnailTask};
use async_trait::async_trait;
use default::DefaultCrawler;
use mangad_neon::CHANNEL_SIZE;
use mangad_neon::core::config::Config;
use mangad_neon::core::entities::dao::crawler::SubTask;
use mangad_neon::core::entities::dao::{SubTaskResult, SubTaskStatus};
use mangad_neon::core::entities::inner::{CanonicalizeTask, ReturningTask};
use mangad_neon::core::entities::orm::sea_orm_active_enums::TaskStatus;
use mangad_neon::core::entities::orm::tasks;
use mangad_neon::core::repository::Repository;
use mangad_neon::error::Error;
use reqwest::Client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio_retry::strategy::ExponentialBackoff;
use tokio::sync::{mpsc, broadcast, oneshot};

mod default;
pub mod jmcomic;

static CACHE_DIRNAME: &str = ".cache";

#[async_trait]
pub trait Crawler: Send + Sync {
    fn site(&self) -> &str;
    async fn handle(&self, subtask: SubTask, client: Arc<Client>) -> Result<Vec<u8>, Error>;
}

pub struct Dispatch {
    config: Arc<Config>,
    client: Arc<Client>,
    default_crawler: Arc<dyn Crawler>,
    clawers: HashMap<String, Arc<dyn Crawler>>,
    semaphore: Arc<Semaphore>,
    storage_root: Arc<PathBuf>,
    canonical_tx: mpsc::Sender<CanonicalizeTask>,
    task_rx: mpsc::Receiver<ReturningTask>,
    delay: u64,
    max_delay: Duration,
    max_retries: usize,

    repo: Arc<Repository>,
    task_tx: broadcast::Sender<tasks::Model>,
    sub_task_tx: broadcast::Sender<SubTaskResult>,
    thumbnail_tx: mpsc::Sender<ThumbnailTask>,
}

impl Dispatch {
    pub fn new(
        config: Arc<Config>,
        repo: Arc<Repository>,
        task_tx: broadcast::Sender<tasks::Model>,
        sub_task_tx: broadcast::Sender<SubTaskResult>,
        thumbnail_tx: mpsc::Sender<ThumbnailTask>,
        canonical_tx: mpsc::Sender<CanonicalizeTask>,
    ) -> (Self, mpsc::Sender<ReturningTask>) {
        let (inner_task_tx, rx) = mpsc::channel::<ReturningTask>(CHANNEL_SIZE);
        let client = Arc::from(Client::new());

        // registry crawlers
        let mut clawers: HashMap<String, Arc<dyn Crawler>> = HashMap::new();

        let jmcomic = JmComicCrawler;
        clawers.insert(jmcomic.site().to_owned(), Arc::new(jmcomic));

        let semaphore = Arc::new(Semaphore::new(config.crawler.semaphore));

        let default_crawler = Arc::new(DefaultCrawler);
        let storage_root = Arc::new(config.crawler.storage.clone());
        let max_delay = Duration::from_secs(config.crawler.retry.max_delay);
        let delay = config.crawler.retry.delay * 1000;
        let max_retries = config.crawler.retry.max_retries;

        let dispatch = Self {
            config,
            clawers,
            client,
            semaphore,
            default_crawler,
            canonical_tx,
            storage_root,
            task_rx: rx,
            delay,
            max_delay,
            max_retries,
            repo,
            task_tx,
            sub_task_tx,
            thumbnail_tx,
        };

        (dispatch, inner_task_tx)
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        let storage_root = self.storage_root.clone();
        'main_loop: while let Some(task) = self.task_rx.recv().await {
            tracing::info!("Received new task: {}", task.task.title());
            let (tx, mut rx) = mpsc::channel::<(i32, Result<PathBuf, Error>)>(1024);
            let ReturningTask { task, tid_tx } = task;
            let Ok(subtasks) = task.split() else {
                tracing::error!("Failed to split subtasks for task: {}", task.title());
                continue 'main_loop;
            };

            let tid = if let Ok(task) = self.repo.insert_task(&task).await {
                task.id
            } else {
                tracing::error!("Unable to insert task into database: {}", task.title());
                continue 'main_loop;
            };

            let _ = tid_tx.send(tid);

            tracing::info!("Task {} inserted with ID: {}", task.title(), tid);

            let format_tid = format!("{:0>10}", tid);

            let cache_at = Arc::new(storage_root.join(CACHE_DIRNAME).join(&format_tid));

            tracing::debug!("Task {} cache at: {:?}", task.title(), cache_at);

            if tokio::fs::create_dir_all(&cache_at.as_ref()).await.is_err() {
                tracing::error!(
                    "Failed to create cache directory for task: {}",
                    task.title()
                );
                continue 'main_loop;
            };

            for subtask in subtasks {
                let clone_semaphore = self.semaphore.clone();
                let clone_crawler = self
                    .clawers
                    .get(&task.source_site)
                    .unwrap_or(&self.default_crawler)
                    .clone();

                let clone_tx = tx.clone();
                let clone_canonical_tx = self.canonical_tx.clone();
                let clone_client = self.client.clone();
                let index = subtask.index;
                let config = Arc::clone(&self.config);
                let base_path = cache_at.clone();
                let strategy = ExponentialBackoff::from_millis(self.delay)
                    .max_delay(self.max_delay)
                    .take(self.max_retries);

                tokio::spawn(async move {
                    let res: Result<PathBuf, Error> = async {
                        let format = if let Some(t) = PathBuf::from(subtask.url.path()).extension()
                        {
                            if let Some(ext) = t.to_str() {
                                Some(ext.to_string())
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        let buffer = tokio_retry::Retry::spawn(strategy, || {
                            let crawler = clone_crawler.clone();
                            let subtask = subtask.clone();
                            let client = clone_client.clone();
                            let semaphore = clone_semaphore.clone();

                            async move {
                                let _permit = semaphore.acquire_owned().await?;
                                crawler.handle(subtask, client).await
                            }
                        })
                        .await?;

                        let (tx, rx) = oneshot::channel();

                        let t = CanonicalizeTask {
                            buffer: Arc::new(buffer),
                            format,
                            base_path,
                            pid: index,
                            quality: config.crawler.image.quality,
                            repeat: tx,
                        };

                        clone_canonical_tx.send(t).await?;

                        rx.await?
                    }
                    .await;

                    if let Err(ref err) = res {
                        tracing::error!("Subtask {} failed: {:?}", index, err);
                    }

                    clone_tx.send((index, res)).await.unwrap();
                });
            }

            drop(tx);

            while let Some((index, res)) = rx.recv().await {
                let clone_sub_task_tx = self.sub_task_tx.clone();
                match res {
                    Ok(_) => {
                        let _ = clone_sub_task_tx.send(SubTaskResult {
                            tid,
                            index,
                            status: SubTaskStatus::Ok,
                        });

                        tracing::debug!("Subtask {} completed successfully", index);
                    }
                    Err(err) => {
                        tracing::error!(
                            "Subtask {} failed, aborting task {}: {:?}",
                            index,
                            tid,
                            err
                        );
                        let _ = clone_sub_task_tx.send(SubTaskResult {
                            tid,
                            index,
                            status: SubTaskStatus::Err(err.to_string()),
                        });

                        let _ = tokio::fs::remove_dir_all(cache_at.as_ref()).await;
                        let model = self
                            .repo
                            .update_task_status_with_reason(tid, TaskStatus::Failure, err)
                            .await?;
                        let _ = self.task_tx.send(model);
                        continue 'main_loop;
                    }
                }
            }

            tracing::info!("All subtasks for task {} downloaded", tid);
            let (id, page_count) = match self.repo.insert_manga_from_task(&task).await {
                Ok(model) => (model.id, model.page_count),
                Err(err) => {
                    eprintln!("failed");
                    let _ = tokio::fs::remove_dir_all(cache_at.as_ref()).await;
                    let model = self
                        .repo
                        .update_task_status_with_reason(tid, TaskStatus::Failure, err)
                        .await?;
                    let _ = self.task_tx.send(model);
                    continue 'main_loop;
                }
            };

            let _ = self
                .thumbnail_tx
                .send(ThumbnailTask {
                    mid: id,
                    r#type: TaskType::Whole(page_count),
                })
                .await;

            let format_mid = format!("{:0>10}", id);
            let storage_at = Arc::new(storage_root.join(&format_mid));
            println!("storage at: {:#?}", storage_at);
            if let Err(err) = tokio::fs::rename(&cache_at.as_ref(), &storage_at.as_ref()).await {
                eprintln!("failed");
                let _ = tokio::fs::remove_dir_all(cache_at.as_ref()).await;
                let model = self
                    .repo
                    .update_task_status_with_reason(tid, TaskStatus::Failure, Error::from(err))
                    .await?;
                let _ = self.task_tx.send(model);
                continue 'main_loop;
            };

            if let Ok(model) = self.repo.update_task_status(tid, TaskStatus::Success).await {
                let _ = self.task_tx.send(model);
            };
        }

        Ok(())
    }
}
