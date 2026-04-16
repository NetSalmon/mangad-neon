pub mod error;
pub mod log;

pub mod core {
    pub mod config;
    pub mod dao;
    pub mod init;
    pub mod orm;
    pub mod repository;
    pub mod token;
}

pub static CHANNEL_SIZE: usize = 1024;
pub static CONFIG_PATH: &str = "./config/config.toml";
