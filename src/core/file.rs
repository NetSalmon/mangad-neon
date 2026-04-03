use crate::error::Error;
use std::path::PathBuf;

pub async fn format(
    bytes: Vec<u8>,
    base_path: PathBuf,
    tid: i32,
    pid: i32,
    quality: f32,
) -> Result<PathBuf, Error> {
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
