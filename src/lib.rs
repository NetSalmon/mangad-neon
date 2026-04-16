pub mod error;
pub mod log;

pub mod core {
    pub mod config;
    pub mod init;
    pub mod repository;
    pub mod token;
    pub mod dao;
    pub mod orm;
}

pub static CHANNEL_SIZE: usize = 1024;
pub static CONFIG_PATH: &str = "./config/config.toml";