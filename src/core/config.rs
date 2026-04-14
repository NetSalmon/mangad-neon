use serde::{Deserialize, Serialize};
use std::path::PathBuf;

macro_rules! default_or_env {
    ($value:ident: PathBuf => $default:expr) => {
        paste::paste! {
            fn [<default_ $value>]() -> PathBuf {
                let env_name = concat!("MANGAD_", stringify!([<$value:upper>]));

                std::env::var(env_name)
                    .map(PathBuf::from)
                    .unwrap_or($default.into())
            }
        }
    };
    ($value:ident: Option<String>) => {
        paste::paste! {
            fn [<default_ $value>]() -> Option<String> {
                let env_name = concat!("MANGAD_", stringify!([<$value:upper>]));
                std::env::var(env_name).ok()
            }
        }
    };
    ($value:ident: String => $default:expr) => {
        paste::paste! {
            fn [<default_ $value>]() -> String {
                let env_name = concat!("MANGAD_", stringify!([<$value:upper>]));
                std::env::var(env_name)
                    .unwrap_or($default)
            }
        }
    };
    ($value:ident: $type:ty => $default:expr) => {
        paste::paste! {
            fn [<default_ $value>]() -> $type {
                let env_name = concat!("MANGAD_", stringify!([<$value:upper>]));

                std::env::var(env_name)
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or($default)
            }
        }
    };
}

default_or_env!(allow_config_remote_read: bool => false);
default_or_env!(allow_config_remote_write: bool => false);

default_or_env!(allow_token_remote_read: bool => false);
default_or_env!(allow_token_remote_create: bool => false);
default_or_env!(allow_token_remote_revoke: bool => false);
default_or_env!(allow_token_remote_modify: bool => false);

default_or_env!(crawler_semaphore: usize => 10);
default_or_env!(crawler_storage: PathBuf => PathBuf::from("./storage"));
default_or_env!(retry_max_retries: usize => 5);
default_or_env!(retry_delay: u64 => 2);
default_or_env!(retry_max_delay: u64 => 30);
default_or_env!(crawler_image_semaphore: usize => 5);
default_or_env!(crawler_image_quality: f32 => 80.0);
default_or_env!(enable_auth: bool => true);
default_or_env!(database_host: String => "127.0.0.1".to_string());
default_or_env!(database_port: u16 => 5432);
default_or_env!(database_name: String => "mangad_neon".to_string());
default_or_env!(database_passwd: String => "".to_string());
default_or_env!(database_user: String => "postgres".to_string());
default_or_env!(service_host: String => "127.0.0.1:6789".to_string());
default_or_env!(thumbnail_quality: f32 => 75.0);
default_or_env!(thumbnail_width: u32 => 200);
default_or_env!(thumbnail_height: u32 => 200);
default_or_env!(thumbnail_enabled: bool => true);
default_or_env!(search_host: String => "http://127.0.0.1:7700".to_string());
default_or_env!(search_api_key: Option<String>);
default_or_env!(log_level: String => "info".to_string());

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
    pub permissions: PermissionsConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PermissionsConfig {
    #[serde(default = "default_allow_config_remote_read")]
    pub allow_config_remote_read: bool,
    #[serde(default = "default_allow_token_remote_read")]
    pub allow_token_remote_read: bool,
    #[serde(default = "default_allow_config_remote_write")]
    pub allow_config_remote_write: bool,
    #[serde(default = "default_allow_token_remote_create")]
    pub allow_token_remote_create: bool,
    #[serde(default = "default_allow_token_remote_revoke")]
    pub allow_token_remote_revoke: bool,
    #[serde(default = "default_allow_token_remote_modify")]
    pub allow_token_remote_modify: bool,
}

impl Default for PermissionsConfig {
    fn default() -> Self {
        Self {
            allow_config_remote_read: default_allow_config_remote_read(),
            allow_token_remote_read: default_allow_token_remote_read(),
            allow_config_remote_write: default_allow_config_remote_write(),
            allow_token_remote_create: default_allow_token_remote_create(),
            allow_token_remote_revoke: default_allow_token_remote_revoke(),
            allow_token_remote_modify: default_allow_token_remote_modify(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CrawlerConfig {
    #[serde(default = "default_crawler_semaphore")]
    pub semaphore: usize,
    #[serde(default = "default_crawler_storage")]
    pub storage: PathBuf,
    #[serde(default)]
    pub retry: RetryConfig,
    #[serde(default)]
    pub image: CrawlerImageConfig,
}

impl Default for CrawlerConfig {
    fn default() -> Self {
        Self {
            semaphore: default_crawler_semaphore(),
            storage: default_crawler_storage(),
            retry: RetryConfig::default(),
            image: CrawlerImageConfig::default(),
        }
    }
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceConfig {
    #[serde(default)]
    pub net: ServiceNetConfig,
    #[serde(default = "default_enable_auth")]
    pub enable_auth: bool,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            net: ServiceNetConfig::default(),
            enable_auth: default_enable_auth(),
        }
    }
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThumbnailConfig {
    #[serde(default = "default_thumbnail_enabled")]
    pub enabled: bool,
    #[serde(default = "default_thumbnail_width")]
    pub width: u32,
    #[serde(default = "default_thumbnail_height")]
    pub height: u32,
    #[serde(default = "default_thumbnail_quality")]
    pub quality: f32,
}

impl Default for ThumbnailConfig {
    fn default() -> Self {
        Self {
            enabled: default_thumbnail_enabled(),
            width: default_thumbnail_width(),
            height: default_thumbnail_height(),
            quality: default_thumbnail_quality(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchConfig {
    #[serde(default = "default_search_host")]
    pub host: String,
    #[serde(default = "default_search_api_key")]
    pub api_key: Option<String>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            host: default_search_host(),
            api_key: default_search_api_key(),
        }
    }
}
