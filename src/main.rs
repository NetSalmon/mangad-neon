extern crate alloc;

use mangad_neon::core::init::init_config;
use mangad_neon::error::Error;
use service::service;

pub mod crawler;
pub mod searching;
pub mod service;

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!(
        "▼ MangaD Neon ▲ \napplication ver: {}",
        env!("CARGO_PKG_VERSION")
    );

    let config = init_config()?;

    if config.service.enable_auth {
        println!("Need HTTPS");
    }

    Ok(service(config).await?)
}
