use image::ImageFormat;
use mangad_neon::core::config::Config;
use mangad_neon::error::Error;
use std::sync::Arc;

pub static THUMBNAIL_PATH: &str = "thumbnail";

pub struct ThumbnailTask {
    pub mid: i32,
    pub page_count: i32,
}

pub struct ThumbnailTaskSingle {
    pub mid: i32,
    pub index: i32,
}

pub async fn thumbnail(
    config: Arc<Config>,
    mut path_rx: tokio::sync::mpsc::Receiver<ThumbnailTask>,
) -> Result<(), Error> {
    'main: while let Some(ref task) = path_rx.recv().await {
        let resp: Result<(), Error> = async {
            let dir = format!("{:0>10}", task.mid);
            let thumbnail_path = config.crawler.storage.join(&dir).join(THUMBNAIL_PATH);
            let storage_path = config.crawler.storage.join(dir);
            tokio::fs::create_dir_all(&thumbnail_path).await?;

            for index in 1..=task.page_count {
                let file = format!("{:0>10}.webp", index);

                let thumbnail_path = thumbnail_path.join(&file);

                let buf = tokio::fs::read(storage_path.join(file)).await?;
                let buf = encode_thumbnail(&config, buf)?;

                tracing::debug!(
                    "thumbnail encoded mid={}, index={}, path={}",
                    task.mid,
                    index,
                    thumbnail_path.display()
                );

                tokio::fs::write(thumbnail_path, buf).await?;
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

pub async fn thumbnail_single(
    config: Arc<Config>,
    mut path_rx: tokio::sync::mpsc::Receiver<ThumbnailTaskSingle>,
) -> Result<(), Error> {
    'main: while let Some(ref task) = path_rx.recv().await {
        let resp: Result<(), Error> = async {
            let dir = format!("{:0>10}", task.mid);
            let thumbnail_path = config.crawler.storage.join(&dir).join(THUMBNAIL_PATH);
            let storage_path = config.crawler.storage.join(dir);
            tokio::fs::create_dir_all(&thumbnail_path).await?;

            let file = format!("{:0>10}.webp", task.index);

            let thumbnail_path = thumbnail_path.join(&file);

            let buf = tokio::fs::read(storage_path.join(file)).await?;
            let buf = encode_thumbnail(&config, buf)?;

            tracing::debug!(
                "thumbnail encoded mid={}, index={}, path={}",
                task.mid,
                task.index,
                thumbnail_path.display()
            );

            tokio::fs::write(thumbnail_path, buf).await?;

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
