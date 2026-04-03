use std::sync::Arc;
use crate::core::entities::config::Config;
use crate::core::service::service;
use crate::error::Error;

pub mod core;
pub mod error;

pub static CONFIG_PATH: &str = "./config/config.toml";

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();

    let config_path = std::env::var("MANGAD_CONFIG_PATH").unwrap_or(CONFIG_PATH.to_string());

    let content = &std::fs::read_to_string(config_path)?;

    let config = toml::from_str::<Config>(content)?;

    println!("CONFIG:\n{:#?}", config);

    Ok(service(Arc::new(config)).await?)
}
