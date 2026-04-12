use std::path::PathBuf;
use image::ImageFormat;
use mangad_neon::core::config::Config;
use mangad_neon::error::Error;
use std::sync::Arc;
use mangad_neon::CHANNEL_SIZE;

pub static THUMBNAIL_PATH: &str = "thumbnail";

pub struct ThumbnailTask {
    pub mid: i32,
    pub r#type: TaskType,
}

pub enum TaskType {
    Single(i32),
    Whole(i32),
}

pub struct Thumbnail {
    config: Arc<Config>,
    task_rx: tokio::sync::mpsc::Receiver<ThumbnailTask>,
}

impl Thumbnail {
    pub fn new(config: Arc<Config>) -> (Self, tokio::sync::mpsc::Sender<ThumbnailTask>) {
        let (tx, task_rx) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
        let thumb = Thumbnail {
            config,
            task_rx
        };
        (thumb, tx)
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        'main: while let Some(ref task) = self.task_rx.recv().await {
            let resp: Result<(), Error> = async {
                let dir = format!("{:0>10}", task.mid);
                let thumbnail_path = self.config.crawler.storage.join(&dir).join(THUMBNAIL_PATH);
                let storage_path = self.config.crawler.storage.join(dir);
                tokio::fs::create_dir_all(&thumbnail_path).await?;

                match task.r#type {
                    TaskType::Single(index) => {
                        self.proc_single(task, &thumbnail_path, &storage_path, index).await?;
                    }
                    TaskType::Whole(page_count) => {
                        for index in 1..=page_count {
                            self.proc_single(task, &thumbnail_path, &storage_path, index).await?;
                        }
                    }
                }

                Ok(())
            }
                .await;

            if let Err(e) = resp {
                tracing::error!("{:?}", e);
                continue 'main;
            }
        }
        Ok(())
    }

    async fn proc_single(&mut self, task: &ThumbnailTask, thumbnail_path: &PathBuf, storage_path: &PathBuf, index: i32) -> Result<(), Error> {
        let file = format!("{:0>10}.webp", index);

        let thumbnail_path = thumbnail_path.join(&file);

        let buf = tokio::fs::read(storage_path.join(file)).await?;
        let buf = encode_thumbnail(&self.config, buf)?;

        tracing::debug!(
            "thumbnail encoded mid={}, index={}, path={}",
            task.mid,
            index,
            thumbnail_path.display()
        );

        tokio::fs::write(thumbnail_path, buf).await?;
        Ok(())
    }
}

fn encode_thumbnail(config: &Arc<Config>, buf: Vec<u8>) -> Result<Vec<u8>, Error> {
    let buf = {
        let image = image::load_from_memory_with_format(&buf, ImageFormat::WebP)?;

        drop(buf);

        let thumb = image.thumbnail(config.thumbnail.width, config.thumbnail.height);

        let width = thumb.width();
        let height = thumb.height();
        let thumb = thumb.into_rgba8();

        let coder = webp::Encoder::from_rgba(&thumb, width, height);
        let buf = coder.encode(config.thumbnail.quality);

        drop(thumb);
        buf.to_vec()
    };
    Ok(buf)
}
