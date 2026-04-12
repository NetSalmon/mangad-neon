use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Config {
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub service: ServiceConfig,
    #[serde(default)]
    pub crawler: CrawlerConfig,
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub thumbnail: ThumbnailConfig,
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub permissions: ConfigPermission,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigPermission {
    #[serde(default = "default_remote_visit")]
    pub allow_remote_visit: bool,
}

impl Default for ConfigPermission {
    fn default() -> Self {
        ConfigPermission {
            allow_remote_visit: default_remote_visit(),
        }
    }
}

fn default_remote_visit() -> bool {
    env::var("MANGAD_ALLOW_REMOTE_VISIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(false)
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: Option<String>,
}

fn default_log_level() -> Option<String> {
    env::var("MANGAD_LOG_LEVEL").ok()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrawlerConfig {
    #[serde(default = "default_crawler_semaphore")]
    pub semaphore: usize,
    #[serde(default = "default_crawler_storage")]
    pub storage: PathBuf,
    #[serde(default = "retry_default")]
    pub retry: RetryConfig,
    #[serde(default)]
    pub image: CrawlerImageConfig,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            semaphore: default_crawler_semaphore(),
            storage: default_crawler_storage(),
            retry: retry_default(),
            image: CrawlerImageConfig::default(),
        }
    }
}

fn default_crawler_semaphore() -> usize {
    env::var("MANGAD_CRAWLER_SEMAPHORE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10)
}

fn default_crawler_storage() -> PathBuf {
    env::var("MANGAD_CRAWLER_STORAGE")
        .map(PathBuf::from)
        .unwrap_or(PathBuf::from("./storage"))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RetryConfig {
    #[serde(default = "default_retry_max_retries")]
    pub max_retries: usize,
    #[serde(default = "default_retry_delay")]
    pub delay: u64,
    #[serde(default = "default_retry_max_delay")]
    pub max_delay: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_retry_max_retries(),
            delay: default_retry_delay(),
            max_delay: default_retry_max_delay(),
        }
    }
}

fn default_retry_max_retries() -> usize {
    env::var("MANGAD_RETRY_MAX_RETRIES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5)
}

fn default_retry_delay() -> u64 {
    env::var("MANGAD_RETRY_DELAY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2)
}

fn default_retry_max_delay() -> u64 {
    env::var("MANGAD_RETRY_MAX_DELAY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30)
}

fn retry_default() -> RetryConfig {
    RetryConfig::default()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrawlerImageConfig {
    #[serde(default = "default_crawler_image_semaphore")]
    pub semaphore: usize,
    #[serde(default = "default_crawler_image_quality")]
    pub quality: f32,
}

impl Default for CrawlerImageConfig {
    fn default() -> Self {
        Self {
            semaphore: default_crawler_image_semaphore(),
            quality: default_crawler_image_quality(),
        }
    }
}

fn default_crawler_image_semaphore() -> usize {
    env::var("MANGAD_CRAWLER_IMAGE_SEMAPHORE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5)
}

fn default_crawler_image_quality() -> f32 {
    env::var("MANGAD_CRAWLER_IMAGE_QUALITY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80.0)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceConfig {
    #[serde(default)]
    pub net: ServiceNetConfig,
    #[serde(default = "default_auth")]
    pub enable_auth: bool,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            net: ServiceNetConfig::default(),
            enable_auth: default_auth(),
        }
    }
}

fn default_auth() -> bool {
    env::var("MANGAD_ENABLE_AUTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(false)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_host")]
    pub host: String,
    #[serde(default = "default_database_port")]
    pub port: u16,
    #[serde(default = "default_database_name")]
    pub database: String,
    #[serde(default = "default_database_user")]
    pub user: String,
    #[serde(default = "default_database_passwd")]
    pub password: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: default_database_host(),
            port: default_database_port(),
            database: default_database_name(),
            user: default_database_user(),
            password: default_database_passwd(),
        }
    }
}

fn default_database_host() -> String {
    env::var("MANGAD_DATABASE_HOST").unwrap_or("127.0.0.1".to_string())
}

fn default_database_port() -> u16 {
    env::var("MANGAD_DATABASE_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5432)
}

fn default_database_name() -> String {
    env::var("MANGAD_DATABASE_NAME").unwrap_or("mangad".to_string())
}

fn default_database_passwd() -> String {
    env::var("MANGAD_DATABASE_PASSWD").unwrap_or("".to_string())
}

fn default_database_user() -> String {
    env::var("MANGAD_DATABASE_USER").unwrap_or("".to_string())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceNetConfig {
    #[serde(default = "default_service_host")]
    pub host: String,
}

impl Default for ServiceNetConfig {
    fn default() -> Self {
        Self {
            host: default_service_host(),
        }
    }
}

fn default_service_host() -> String {
    env::var("MANGAD_SERVICE_HOST").unwrap_or("127.0.0.1:6789".to_string())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThumbnailConfig {
    #[serde(default = "thumbnail_default_enabled")]
    pub enabled: bool,
    #[serde(default = "thumbnail_default_width")]
    pub width: u32,
    #[serde(default = "thumbnail_default_height")]
    pub height: u32,
    #[serde(default = "thumbnail_default_quality")]
    pub quality: f32,
}

impl Default for ThumbnailConfig {
    fn default() -> Self {
        Self {
            enabled: thumbnail_default_enabled(),
            width: thumbnail_default_width(),
            height: thumbnail_default_height(),
            quality: thumbnail_default_quality(),
        }
    }
}

fn thumbnail_default_quality() -> f32 {
    env::var("MANGAD_THUMBNAIL_QUALITY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(75.0)
}
fn thumbnail_default_width() -> u32 {
    env::var("MANGAD_THUMBNAIL_WIDTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200)
}
fn thumbnail_default_height() -> u32 {
    env::var("MANGAD_THUMBNAIL_HEIGHT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(200)
}
fn thumbnail_default_enabled() -> bool {
    env::var("MANGAD_THUMBNAIL_ENABLED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(true)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchConfig {
    #[serde(default = "default_search_host")]
    pub host: String,
    #[serde(default = "default_api_key")]
    pub api_key: Option<String>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            host: default_search_host(),
            api_key: default_api_key(),
        }
    }
}

fn default_search_host() -> String {
    env::var("MANGAD_SEARCH_HOST").unwrap_or("http://127.0.0.1:7700".to_string())
}

fn default_api_key() -> Option<String> {
    env::var("MANGAD_SEARCH_API_KEY").ok()
}
