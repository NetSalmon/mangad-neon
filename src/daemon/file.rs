use std::path::PathBuf;
use crate::daemon::models::errors::DaemonError;

pub async fn replace(from_cache: &PathBuf, to_storage: &PathBuf) -> Result<(), DaemonError> {
    let err = tokio::fs::rename(from_cache, to_storage).await;
    if err.is_err() {
        tracing::debug!(
            "replace: failed to rename {} to {}",
            from_cache.display(),
            to_storage.display()
        );
        tokio::fs::create_dir(to_storage).await?;
        let mut images = tokio::fs::read_dir(from_cache).await?;
        while let Some(entry) = images.next_entry().await? {
            let Ok(filename) = entry.file_name().into_string() else {
                continue;
            };
            let from = from_cache.join(&filename);
            let to = to_storage.join(&filename);
            if tokio::fs::copy(&from, &to).await.is_err() {
                tracing::debug!(
                    "replace: failed to copy {} to {}",
                    from.display(),
                    to.display()
                );
                continue;
            };
        }
        tokio::fs::remove_dir_all(to_storage).await?;
        Ok(())
    } else {
        Ok(())
    }
}
