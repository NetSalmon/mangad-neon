use crate::daemon::models::error::AppError;
use crate::daemon::service::service;
use mangad_neon::core::init::init_config;
use mangad_neon::log;

mod daemon;
#[tokio::main]
async fn main() -> Result<(), AppError> {
    let (path, config) = init_config()?;

    log::init(&config.log);

    tracing::info!(
        "▼ MangaD Neon ▲ \napplication ver: {}",
        env!("CARGO_PKG_VERSION")
    );

    if config.service.enable_auth {
        tracing::warn!("Need HTTPS for enabled auth");
    }

    Ok(service(config, path).await?)
}
