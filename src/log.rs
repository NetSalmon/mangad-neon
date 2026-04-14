use crate::core::config::LogConfig;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init(config: &LogConfig) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true))
        .init();

    tracing::info!("Logging initialized");
}
