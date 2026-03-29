use tokio::sync::mpsc::Receiver;
use crate::core::entities::inner::{CanonicalizeResult, CanonicalizeTask};
use crate::error::Error;

pub struct Canonicalization {
    pub rx: Receiver<CanonicalizeTask>,
}

impl Canonicalization {
    pub async fn run(&mut self) {
        while let Some(can) = self.rx.recv().await {
            tokio::spawn(async move {
                let result: CanonicalizeResult = async {
                    let encoded_data: Vec<u8> = {
                        let image = image::load_from_memory(&can.buffer)?.to_rgba8();
                        let encoder = webp::Encoder::from_rgba(&image, image.width(), image.height());
                        let mem = encoder.encode(can.quality);

                        mem.to_vec()
                    };

                    let target = can.base_path.join(format!("{:0>10}", can.tid));

                    tokio::fs::create_dir_all(&target).await?;

                    let file_path = target.join(format!("{:0>10}.webp", can.pid));

                    tokio::fs::write(&file_path, &encoded_data).await?;

                    Ok(file_path)
                }.await;

                let _ = can.repeat.send(result);
            });
        }
    }
}