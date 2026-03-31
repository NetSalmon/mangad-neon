use crate::core::entities::config::Config;
use crate::core::entities::dao::crawler::{SubTask, Task};
use crate::core::entities::inner::{CanonicalizeResult, CanonicalizeTask};
use crate::core::entities::orm::sea_orm_active_enums::TaskStatus;
use crate::core::image::Canonicalization;
use crate::core::repository::Repository;
use crate::error::Error;
use async_trait::async_trait;
use default::DefaultClawer;
use reqwest::Client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

mod default;
pub mod nhentai;

static CACHE_DIRNAME: &str = ".cache";

#[async_trait]
pub trait Crawler: Send + Sync {
    fn site(&self) -> &str;
    async fn handle(&self, subtask: SubTask, client: Arc<Client>) -> Result<Vec<u8>, Error>;
}

pub struct Dispatch {
    config: Arc<Config>,
    client: Arc<Client>,
    retry: Option<Retry>,
    default_crawler: Arc<dyn Crawler>,
    clawers: HashMap<String, Arc<dyn Crawler>>,
    semaphore: Arc<Semaphore>,
    storage_root: Arc<PathBuf>,
    canonical_tx: tokio::sync::mpsc::Sender<CanonicalizeTask>,
    rx: tokio::sync::mpsc::Receiver<Task>,
}

#[derive(Copy, Clone)]
pub struct Retry {
    max_retries: usize,
    delay: Duration,
}

impl Dispatch {
    pub fn new(config: Arc<Config>) -> (Self, tokio::sync::mpsc::Sender<Task>) {
        let (tx, rx) = tokio::sync::mpsc::channel::<Task>(1024);
        let (canonical_tx, canonical_rx) = tokio::sync::mpsc::channel::<CanonicalizeTask>(1024);
        let client = Arc::from(Client::new());
        let clawers = HashMap::new();
        let semaphore = Arc::new(Semaphore::new(config.crawler.semaphore));
        let canonical_semaphore = Arc::new(Semaphore::new(config.crawler.image.semaphore));

        let retry = if let Some(ref cfg) = config.crawler.retry {
            let retry = Retry {
                max_retries: cfg.max_retries,
                delay: Duration::from_secs(cfg.delay),
            };
            Some(retry)
        } else {
            None
        };

        let mut canonicalization = Canonicalization::new(canonical_rx, canonical_semaphore);

        tokio::spawn(async move {
            canonicalization.run().await;
        });

        let default_crawler = Arc::new(DefaultClawer {});
        let storage_root = Arc::new(config.crawler.storage.clone());

        let dispatch = Self {
            config,
            clawers,
            client,
            semaphore,
            default_crawler,
            canonical_tx,
            storage_root,
            retry,
            rx,
        };

        (dispatch, tx)
    }

    pub async fn run(&mut self, repo: Arc<Repository>) -> Result<(), Error> {
        let storage_root = self.storage_root.clone();
        while let Some(task) = self.rx.recv().await {
            let (tx, mut rx) =
                tokio::sync::mpsc::channel::<(i32, Result<CanonicalizeResult, Error>)>(1024);
            let subtasks = task.spilt()?;

            let tid = repo.insert_task(&task).await?.id;
            let format_tid = format!("{:0>10}", tid);

            let storage_at = Arc::new(storage_root.join(&format_tid));

            println!("storage at: {:#?}", storage_at);

            let cache_at = Arc::new(storage_root.join(CACHE_DIRNAME).join(&format_tid));

            println!("cache at: {:#?}", cache_at);

            tokio::fs::create_dir_all(&cache_at.as_ref()).await?;

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
                let retry_setting = self.retry.clone();
                let index = subtask.index;
                let config = Arc::clone(&self.config);
                let base_path = cache_at.clone();

                tokio::spawn(async move {
                    let res: Result<CanonicalizeResult, Error> = async {
                        let _permit = clone_semaphore.acquire_owned().await;
                        let buffer = if let Some(r) = retry_setting {
                            retry(
                                || clone_crawler.handle(subtask.clone(), clone_client.clone()),
                                r.max_retries,
                                r.delay,
                            )
                            .await?
                        } else {
                            clone_crawler.handle(subtask, clone_client.clone()).await?
                        };

                        let (tx, rx) = tokio::sync::oneshot::channel();

                        let t = CanonicalizeTask {
                            buffer,
                            base_path,
                            pid: index,
                            quality: config.crawler.image.quality,
                            repeat: tx,
                        };

                        clone_canonical_tx.send(t).await?;

                        Ok(rx.await?)
                    }
                    .await;

                    clone_tx.send((index, res)).await.unwrap();
                });
            }

            drop(tx);

            while let Some((index, res)) = rx.recv().await {
                print!("subtask [{}] status: ", index);
                match res {
                    Ok(_) => {
                        println!("done");
                    }
                    Err(_) => {
                        eprintln!("failed");
                    }
                }
            }

            tokio::fs::rename(&cache_at.as_ref(), &storage_at.as_ref()).await?;
            repo.update_task_status(tid, TaskStatus::Success).await?;
            repo.insert_manga_from_task(&task).await?;
        }

        Ok(())
    }
}

async fn retry<F, Fut, T>(func: F, max_retry: usize, gap: Duration) -> Result<T, Error>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    let mut gap = gap;
    let mut last_error = None;
    for _ in 1..max_retry {
        match func().await {
            Ok(r) => return Ok(r),
            Err(Error::RequestError(e)) => {
                last_error = Some(Error::RequestError(e));
                tokio::time::sleep(gap).await;
                gap *= 2;
            }
            Err(e) => return Err(e),
        }
    }

    Err(last_error.unwrap_or(Error::MaxRetriesError("超出最大重试次数".into())))
}
