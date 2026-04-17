use crate::CONFIG_PATH;
use crate::config::Config;
use crate::error::Error;
use std::path::PathBuf;

pub fn init_config() -> Result<(PathBuf, Config), Error> {
    dotenvy::dotenv().ok();

    let config_path = std::env::var("MANGAD_CONFIG_PATH").unwrap_or(CONFIG_PATH.to_string());
    let path = PathBuf::from(config_path);

    let config = if path.exists() {
        let content = &std::fs::read_to_string(&path)?;
        toml::from_str::<Config>(content)?
    } else {
        tracing::warn!(
            "Config file not found at {:?}, using defaults/env vars",
            path
        );
        Config::default()
    };

    Ok((path, config))
}
