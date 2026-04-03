pub mod handlers;
pub mod router;

use super::entities::config::Config;
use crate::Error;
use axum::routing::get;

pub async fn service(config: Config) -> Result<(), Error> {
    let addr = tokio::net::TcpListener::bind(config.service.net.host).await?;

    let router = axum::Router::new().route("/health", get(handlers::basic::health));

    let app = axum::serve(addr, router).await?;

    Ok(app)
}
