use crate::core::config::LogConfig;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init(config: &LogConfig) {
    let filter = if let Some(ref level) = config.level {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level))
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true))
        .init();

    tracing::info!("Logging initialized");
}
