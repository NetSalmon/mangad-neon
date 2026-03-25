use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub service: ServiceConfig,
    pub clawer: ClawerConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClawerConfig {
    semaphore: u64,
    retry: Option<RetryConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RetryConfig {
    max_retries: u64,
    delay: u64,
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
    pub enabled: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
