use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub service: ServiceConfig,
    pub crawler: CrawlerConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CrawlerConfig {
    pub semaphore: u64,
    pub storage: PathBuf,
    pub retry: Option<RetryConfig>,
    pub image: CrawlerImageConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub delay: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CrawlerImageConfig {
    pub quality: f32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceConfig {
    pub net: ServiceNetConfig,
    pub image: ImageConfig,
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

fn thumbnail_default_quality() -> f32 { 75.0 }
fn thumbnail_default_size() -> u32 { 200 }
fn thumbnail_default_enabled() -> bool { true }
