use crate::core::entities::inner::{CanonicalizeResult, CanonicalizeTask};
use crate::error::Error;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::sync::mpsc::Receiver;

pub struct Canonicalization {
    pub rx: Receiver<CanonicalizeTask>,
    pub semaphore: Arc<Semaphore>,
}

impl Canonicalization {
    pub fn new(rx: Receiver<CanonicalizeTask>, semaphore: Arc<Semaphore>) -> Self {
        Self { rx, semaphore }
    }

    pub async fn run(&mut self) {
        while let Some(can) = self.rx.recv().await {
            let semaphore = self.semaphore.clone();
            tokio::spawn(async move {
                let result: CanonicalizeResult = async {
                    let _permit = semaphore.acquire_owned().await;
                    let encoded_result =
                        tokio::task::spawn_blocking(move || -> Result<Vec<u8>, Error> {
                            let image = if let Some(format) = can.format {
                                if let Some(f) = image::ImageFormat::from_extension(&format) {
                                    image::load_from_memory_with_format(&can.buffer, f)?.to_rgba8()
                                } else {
                                    image::load_from_memory(&can.buffer)?.to_rgba8()
                                }
                            } else {
                                image::load_from_memory(&can.buffer)?.to_rgba8()
                            };

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
                            return Err(Error::TaskJoinError(e));
                        }
                    };

                    let file_path = can.base_path.join(format!("{:0>10}.webp", can.pid));

                    tokio::fs::write(&file_path, &encoded_data).await?;

                    Ok(file_path)
                }
                .await;

                let _ = can.repeat.send(result);
            });
        }
    }
}
