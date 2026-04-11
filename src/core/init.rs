use crate::core::config::Config;
use crate::error::Error;
use std::path::PathBuf;

pub static CONFIG_PATH: &str = "./config/config.toml";

pub fn init_config() -> Result<(PathBuf, Config), Error> {
    dotenvy::dotenv().ok();

    let config_path = std::env::var("MANGAD_CONFIG_PATH").unwrap_or(CONFIG_PATH.to_string());
    let path = PathBuf::from(config_path);

    let content = &std::fs::read_to_string(&path)?;

    let config = toml::from_str::<Config>(content)?;

    Ok((path, config))
}
