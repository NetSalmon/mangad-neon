use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub database: DatabaseConfig,
    pub service: ServiceConfig,
    pub crawler: CrawlerConfig,
    pub search: SearchConfig,
    #[serde(default)]
    pub log: LogConfig,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct LogConfig {
    pub level: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CrawlerConfig {
    pub semaphore: usize,
    pub storage: PathBuf,
    #[serde(default = "retry_default")]
    pub retry: RetryConfig,
    pub image: CrawlerImageConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub delay: u64,
    pub max_delay: u64,
}

fn retry_default() -> RetryConfig {
    RetryConfig {
        max_delay: 30,
        delay: 2,
        max_retries: 5,
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CrawlerImageConfig {
    pub semaphore: usize,
    pub quality: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceConfig {
    pub net: ServiceNetConfig,
    pub image: ImageConfig,
    #[serde(default = "default_auth")]
    pub enable_auth: bool,
}

fn default_auth() -> bool {
    false
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ImageConfig {
    pub thumbnail: ThumbnailConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceNetConfig {
    pub host: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ThumbnailConfig {
    #[serde(default = "thumbnail_default_enabled")]
    pub enabled: bool,
    #[serde(default = "thumbnail_default_size")]
    pub width: u32,
    #[serde(default = "thumbnail_default_size")]
    pub height: u32,
    #[serde(default = "thumbnail_default_quality")]
    pub quality: f32,
}

fn thumbnail_default_quality() -> f32 {
    75.0
}
fn thumbnail_default_size() -> u32 {
    200
}
fn thumbnail_default_enabled() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchConfig {
    pub host: String,
    pub api_key: Option<String>,
}
