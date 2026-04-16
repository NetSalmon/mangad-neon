pub mod common;

pub use common::{error, logger, db, token, init, config};

pub static CHANNEL_SIZE: usize = 1024;
pub static CONFIG_PATH: &str = "./config/config.toml";
