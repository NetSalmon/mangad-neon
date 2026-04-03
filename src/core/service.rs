pub mod handlers;
pub mod router;

use super::entities::config::Config;
use crate::Error;
use crate::core::crawler::Dispatch;
use crate::core::entities::inner::InnerTask;
use crate::core::repository::Repository;
use axum::routing::{get, post};
use std::sync::Arc;

pub struct AppState {
    pub config: Arc<Config>,
    pub crawler_tx: Arc<tokio::sync::mpsc::Sender<InnerTask>>,
}

pub async fn service(config: Arc<Config>) -> Result<(), Error> {
    let (mut dispatch, tx) = Dispatch::new(config.clone());
    let repo = Arc::new(Repository::new(config.clone()).await?);

    tokio::spawn(async move {
        let _ = dispatch.run(repo).await;
    });

    let state = Arc::new(AppState {
        config: config.clone(),
        crawler_tx: Arc::new(tx),
    });

    let addr = tokio::net::TcpListener::bind(&config.service.net.host).await?;

    let router = axum::Router::new()
        .route("/health", get(handlers::basic::health))
        .route("/tasks", post(handlers::business::add_tasks))
        .with_state(state.clone());

    let app = axum::serve(addr, router).await?;

    Ok(app)
}
