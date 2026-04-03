use crate::core::entities::config::Config;
use crate::core::entities::dao::crawler::SubTask;
use crate::core::entities::inner::{CanonicalizeResult, CanonicalizeTask, InnerTask};
use crate::core::entities::orm::sea_orm_active_enums::TaskStatus;
use crate::core::image::Canonicalization;
use crate::core::repository::Repository;
use crate::error::Error;
use async_trait::async_trait;
use default::DefaultCrawler;
use reqwest::Client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio_retry::strategy::ExponentialBackoff;

mod default;

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
    canonical_tx: tokio::sync::mpsc::Sender<CanonicalizeTask>,
    rx: tokio::sync::mpsc::Receiver<InnerTask>,
    delay: u64,
    max_delay: Duration,
    max_retries: usize,
}

impl Dispatch {
    pub fn new(config: Arc<Config>) -> (Self, tokio::sync::mpsc::Sender<InnerTask>) {
        let (tx, rx) = tokio::sync::mpsc::channel::<InnerTask>(1024);
        let (canonical_tx, canonical_rx) = tokio::sync::mpsc::channel::<CanonicalizeTask>(1024);
        let client = Arc::from(Client::new());
        let clawers = HashMap::new();
        let semaphore = Arc::new(Semaphore::new(config.crawler.semaphore));
        let canonical_semaphore = Arc::new(Semaphore::new(config.crawler.image.semaphore));

        let mut canonicalization = Canonicalization::new(canonical_rx, canonical_semaphore);

        tokio::spawn(async move {
            canonicalization.run().await;
        });

        let default_crawler = Arc::new(DefaultCrawler {});
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
            rx,
            delay,
            max_delay,
            max_retries,
        };

        (dispatch, tx)
    }

    pub async fn run(&mut self, repo: Arc<Repository>) -> Result<(), Error> {
        let storage_root = self.storage_root.clone();
        'main_loop: while let Some(task) = self.rx.recv().await {
            let (tx, mut rx) =
                tokio::sync::mpsc::channel::<(i32, Result<CanonicalizeResult, Error>)>(1024);
            let (task, id_tx) = (task.task, task.id_tx);
            let Ok(subtasks) = task.split() else {
                eprintln!("failed to split subtasks");
                continue 'main_loop;
            };

            let tid = if let Ok(task) = repo.insert_task(&task).await {
                task.id
            } else {
                eprintln!("unable to insert task");
                continue 'main_loop;
            };

            let _ = id_tx.send(tid); // 返回task id

            let format_tid = format!("{:0>10}", tid);

            let cache_at = Arc::new(storage_root.join(CACHE_DIRNAME).join(&format_tid));

            println!("cache at: {:#?}", cache_at);

            if tokio::fs::create_dir_all(&cache_at.as_ref()).await.is_err() {
                eprintln!("failed to create cache dir");
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
                    let res: Result<CanonicalizeResult, Error> = async {
                        let buffer = tokio_retry::Retry::spawn(strategy, || {
                            println!("testing");
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

                        let (tx, rx) = tokio::sync::oneshot::channel();

                        let t = CanonicalizeTask {
                            buffer: Arc::new(buffer),
                            base_path,
                            pid: index,
                            quality: config.crawler.image.quality,
                            repeat: tx,
                        };

                        clone_canonical_tx.send(t).await?;

                        Ok(rx.await?)
                    }
                    .await;

                    println!("result: {:#?}", res);

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
                    Err(err) => {
                        eprintln!("failed");
                        let _ = tokio::fs::remove_dir_all(cache_at.as_ref()).await;
                        repo.update_task_status_with_reason(tid, TaskStatus::Failure, err)
                            .await?;
                        continue 'main_loop;
                    }
                }
            }

            println!("all download done");
            let id = match repo.insert_manga_from_task(&task).await {
                Ok(model) => model.id,
                Err(err) => {
                    eprintln!("failed");
                    let _ = tokio::fs::remove_dir_all(cache_at.as_ref()).await;
                    repo.update_task_status_with_reason(tid, TaskStatus::Failure, err)
                        .await?;
                    continue 'main_loop;
                }
            };

            let format_mid = format!("{:0>10}", id);
            let storage_at = Arc::new(storage_root.join(&format_mid));
            println!("storage at: {:#?}", storage_at);
            if let Err(err) = tokio::fs::rename(&cache_at.as_ref(), &storage_at.as_ref()).await {
                eprintln!("failed");
                let _ = tokio::fs::remove_dir_all(cache_at.as_ref()).await;
                repo.update_task_status_with_reason(tid, TaskStatus::Failure, Error::from(err))
                    .await?;
                continue 'main_loop;
            };
            let _ = repo.update_task_status(tid, TaskStatus::Success).await;
        }

        Ok(())
    }
}
