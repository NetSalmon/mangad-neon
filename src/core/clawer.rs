use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use nix::libc::sleep;
use reqwest::Client;
use tokio::sync::Semaphore;
use crate::core::entities::dao::clawer::{SubTask, Task};
use crate::error::Error;

pub mod nhentai;

#[async_trait]
pub trait Clawer: Send + Sync {
    fn site(&self) -> &str;
    async fn handle(&self, subtask: &SubTask, client: Arc<Client>) -> Result<(), Error>;
}

pub struct Dispatch {
    client: Arc<Client>,
    clawers: HashMap<String, Arc<dyn Clawer>>,
    semaphore: Arc<Semaphore>,
    rx: tokio::sync::mpsc::Receiver<Task>,
}

impl Dispatch {
    pub fn new() -> (Self, tokio::sync::mpsc::Sender<Task>) {
        let (tx, rx) = tokio::sync::mpsc::channel::<Task>(1024);
        let client = Arc::from(Client::new());
        let clawers = HashMap::new();
        let semaphore = Arc::new(Semaphore::new(5));

        let dispatch = Self {
            clawers,
            client,
            semaphore,
            rx
        };

        (dispatch, tx)
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        loop {
            let Some(task) = self.rx.recv().await else {
                break;
            };

            if let Some(clawer) = self.clawers.get(&task.source_site) {
                // clawer.handle(&task, Arc::clone(&self.client)).await?;
            } else {
                // 使用 DefaultClawer
            }
        }

        Ok(())
    }
}

async fn retry<F, Fut, T>(func: F, max_retry: usize, gap: Duration) -> Result<T, Error>
where F: Fn() -> Fut,
    Fut: Future<Output = Result<T, Error>>,
{
    let mut gap = gap;
    let mut last_error = None;
    for i in 1..max_retry {
        match func().await {
            Ok(r) => return Ok(r),
            Err(Error::RequestError(e)) => {
                last_error = Some(Error::RequestError(e));
                tokio::time::sleep(gap).await;
                gap *= 2;
            },
            Err(e) => return Err(e),
        }
    }

    Err(last_error.unwrap_or(Error::MaxRetriesError("超出最大重试次数".into())))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_retry() {
        async fn a(i: &str) -> Result<(), Error> {
            println!("a: {}", i);
            Err(Error::TestingError("error".into()))
        }

        retry(|| { a("123") }, 5, Duration::from_secs(1)).await.unwrap();
    }
}