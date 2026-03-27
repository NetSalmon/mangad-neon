use crate::error::Error;
use image::EncodableLayout;
use std::path::PathBuf;

pub async fn format(bytes: Vec<u8>, base_path: PathBuf, tid: i32, pid: i32, quality: f32) -> Result<PathBuf, Error> {
    let encoded_data: Vec<u8> = {
        let image = image::load_from_memory(&bytes)?.to_rgba8();
        let encoder = webp::Encoder::from_rgba(&image, image.width(), image.height());
        let mem = encoder.encode(quality);

        mem.to_vec()
    };

    let target = base_path.join(format!("{:0>10}", tid));

    tokio::fs::create_dir_all(&target).await?;

    let file_path = target.join(format!("{:0>10}.webp", pid));

    tokio::fs::write(&file_path, &encoded_data).await?;

    Ok(file_path)
}

pub async fn move_dir(from: &PathBuf, to: &PathBuf, mid: i32) -> Result<PathBuf, Error> {
    let dist_path = to.join(format!("{:0>10}", mid));
    println!("{} {}", from.display(), dist_path.display());
    tokio::fs::create_dir_all(&dist_path).await?;
    tokio::fs::rename(&from, &dist_path).await?;

    Ok(dist_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Arc;
    use std::time::Duration;
    use rand::{random_range, RngExt};
    use tokio::sync::Semaphore;

    #[tokio::test]
    async fn test_move_dir() {
        let from = PathBuf::from("test/0000000001");
        let to = PathBuf::from("test/");
        move_dir(&from, &to, 2).await.unwrap();
    }

    #[tokio::test]
    async fn test_format() {
        let bytes = fs::read("test/0244C2C7C8CA66C64D052DF9A307A26C.jpg").unwrap();
        let base_path = PathBuf::from("test/");

        format(bytes, base_path, 1, 123, 100.0).await.unwrap();
    }

    #[tokio::test]
    async fn test_move_ok() {
        let sem = Arc::new(Semaphore::new(3));
        let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(32);
        tokio::spawn(async move {
            for i in 0..10 {
                let c_sem = sem.clone();
                let c_tx = tx.clone();

                tokio::spawn(async move {
                    let _permit = c_sem.acquire().await.unwrap();
                    tokio::time::sleep(Duration::from_secs(i)).await;
                    c_tx.send(format!("Hello {}", i)).await.unwrap();
                });
            }
        });

        while let Some(t) = rx.recv().await {
            println!("{}", t);
        }

        println!("所有任务完成完毕");
    }
}
