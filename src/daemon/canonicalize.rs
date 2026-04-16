use crate::daemon::models::error::AppError;
use crate::daemon::models::tasks::CanonicalizeResult;
use crate::daemon::models::tasks::CanonicalizeTask;
use mangad_neon::CHANNEL_SIZE;
use mangad_neon::core::config::Config;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::sync::mpsc;

pub struct Canonicalization {
    pub rx: mpsc::Receiver<CanonicalizeTask>,
    pub semaphore: Arc<Semaphore>,
}

impl Canonicalization {
    pub fn new(config: Arc<Config>) -> (Self, mpsc::Sender<CanonicalizeTask>) {
        let semaphore = Arc::new(Semaphore::new(config.crawler.image.semaphore));
        let (tx, rx) = mpsc::channel::<CanonicalizeTask>(CHANNEL_SIZE);

        (Self { rx, semaphore }, tx)
    }

    pub async fn run(&mut self) {
        while let Some(can) = self.rx.recv().await {
            let semaphore = self.semaphore.clone();
            tokio::spawn(async move {
                let result: CanonicalizeResult = async {
                    let _permit = semaphore.acquire_owned().await;
                    let encoded_result =
                        tokio::task::spawn_blocking(move || -> Result<Vec<u8>, AppError> {
                            let format = can
                                .format
                                .as_ref()
                                .and_then(|f| image::ImageFormat::from_extension(f));

                            let dynamic_image = match format {
                                Some(f) => image::load_from_memory_with_format(&can.buffer, f)?,
                                None => image::load_from_memory(&can.buffer)?,
                            };

                            let image = dynamic_image.to_rgba8();

                            let encoder =
                                webp::Encoder::from_rgba(&image, image.width(), image.height());
                            let mem = encoder.encode(can.quality);

                            drop(image);

                            Ok(mem.to_vec())
                        })
                        .await;

                    drop(_permit);

                    let encoded_data = match encoded_result {
                        Ok(Ok(data)) => data,
                        Ok(Err(e)) => {
                            return Err(e);
                        }
                        Err(e) => {
                            return Err(AppError::TaskJoinError(e));
                        }
                    };

                    let file_path = can.base_path.join(format!("{:0>10}.webp", can.pid + 1)); // start from 1

                    tokio::fs::write(&file_path, &encoded_data).await?;

                    Ok(file_path)
                }
                .await;

                let _ = can.repeat.send(result);
            });
        }
    }
}
