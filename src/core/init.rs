use crate::core::config::Config;
use crate::error::Error;
use std::sync::Arc;

pub static CONFIG_PATH: &str = "./config/config.toml";

pub fn init_config() -> Result<Arc<Config>, Error> {
    dotenvy::dotenv().ok();

    let config_path = std::env::var("MANGAD_CONFIG_PATH").unwrap_or(CONFIG_PATH.to_string());

    let content = &std::fs::read_to_string(config_path)?;

    let config = toml::from_str::<Config>(content)?;

    Ok(Arc::new(config))
}
