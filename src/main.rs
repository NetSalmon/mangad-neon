use crate::daemon::models::errors::DaemonError;
use crate::daemon::service::service;
use mangad_neon::init::init_config;
use mangad_neon::logger;

mod daemon;
#[tokio::main]
async fn main() -> Result<(), DaemonError> {
    let (path, config) = init_config()?;

    logger::init(&config.log);

    tracing::info!(
        "▼ MangaD Neon ▲ \napplication ver: {}",
        env!("CARGO_PKG_VERSION")
    );

    if config.service.enable_auth {
        tracing::warn!("Need HTTPS for enabled auth");
    }

    Ok(service(config, path).await?)
}
