use crate::core::entities::config::Config;
use crate::core::entities::dao::crawler::{SubTask, Task};
use crate::core::file::format;
use crate::error::Error;
use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use default::DefaultClawer;

pub mod nhentai;
mod default;

static CACHE_DIRNAME: &str = ".cache";

#[async_trait]
pub trait Crawler: Send + Sync {
    fn site(&self) -> &str;
    async fn handle(&self, subtask: SubTask, client: Arc<Client>) -> Result<Vec<u8>, Error>;
}

pub struct Dispatch {
    client: Arc<Client>,
    retry: Option<Retry>,
    default_crawler: Arc<dyn Crawler>,
    clawers: HashMap<String, Arc<dyn Crawler>>,
    semaphore: Arc<Semaphore>,
    storage_root: PathBuf,
    rx: tokio::sync::mpsc::Receiver<Task>,
}

#[derive(Copy, Clone)]
pub struct Retry {
    max_retries: usize,
    delay: Duration,
}

impl Dispatch {
    pub fn new(config: &Config) -> (Self, tokio::sync::mpsc::Sender<Task>) {
        let (tx, rx) = tokio::sync::mpsc::channel::<Task>(1024);
        let client = Arc::from(Client::new());
        let clawers = HashMap::new();
        let semaphore = Arc::new(Semaphore::new(5));

        let retry = if let Some(ref cfg) = config.crawler.retry {
            let retry = Retry {
                max_retries: cfg.max_retries,
                delay: Duration::from_secs(cfg.delay),
            };
            Some(retry)
        } else {
            None
        };

        let default_crawler = Arc::new(DefaultClawer {});

        let dispatch = Self {
            clawers,
            client,
            semaphore,
            default_crawler,
            storage_root: config.crawler.storage.clone(),
            retry,
            rx,
        };

        (dispatch, tx)
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        let storage_root = self.storage_root.clone();
        while let Some(task) = self.rx.recv().await {
            let (tx, mut rx) = tokio::sync::mpsc::channel::<(usize, Result<(), Error>)>(1024);
            let subtasks = task.spilt()?;
            let storage_at = storage_root.join(format!("{:0>10}", task.id));
            let cache_at = storage_root.join(CACHE_DIRNAME).join(format!("{:0>10}", task.id));

            for subtask in subtasks {
                let clone_semaphore = self.semaphore.clone();
                let clone_crawler = self.clawers
                    .get(&task.source_site)
                    .unwrap_or(&self.default_crawler)
                    .clone();

                let clone_tx = tx.clone();
                let clone_client = self.client.clone();
                let retry_setting = self.retry.clone();
                let index = subtask.index;

                let base_path = PathBuf::from_str("test")?;

                tokio::spawn(async move {
                    let res: Result<(), Error> = async {
                        let _permit = clone_semaphore.acquire_owned().await;
                        let buffer = if let Some(r) = retry_setting {
                            retry(
                                || clone_crawler.handle(subtask.clone(), clone_client.clone()),
                                r.max_retries,
                                r.delay,
                            ).await?
                        } else {
                            clone_crawler.handle(subtask, clone_client.clone()).await?
                        };

                        format(buffer, base_path, 0, 0, 100.0).await?;
                        Ok(())
                    }.await;

                    clone_tx.send((index, res)).await.unwrap();
                });
            }

            drop(tx);

            while let Some((index, res)) = rx.recv().await {
                print!("subtask [{}] status: ", index);
                match res {
                    Ok(_) => { println!("done"); },
                    Err(_) => { eprintln!("failed"); },
                }
            }
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

async fn query_tid(task: &Task) -> Result<i32, Error> {
    todo!()
}
