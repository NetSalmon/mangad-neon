extern crate alloc;

use mangad_neon::core::init::init_config;
use mangad_neon::error::Error;
use mangad_neon::log;
use service::service;

pub mod canonicalize;
pub mod crawler;
pub mod file;
pub mod searching;
pub mod service;
pub mod thumbnail;

#[tokio::main]
async fn main() -> Result<(), Error> {
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
