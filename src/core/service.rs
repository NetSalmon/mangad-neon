pub mod handlers;
pub mod middleware;

use super::entities::config::Config;
use crate::Error;
use crate::core::crawler::Dispatch;
use crate::core::entities::inner::InnerTask;
use crate::core::repository::Repository;
use axum::middleware::from_fn_with_state;
use axum::routing::{get, patch, post};
use std::sync::Arc;

pub struct AppState {
    pub config: Arc<Config>,
    pub repo: Arc<Repository>,
    pub crawler_tx: Arc<tokio::sync::mpsc::Sender<InnerTask>>,
}

pub async fn service(config: Arc<Config>) -> Result<(), Error> {
    let (mut dispatch, tx) = Dispatch::new(config.clone());
    let repo = Arc::new(Repository::new(config.clone()).await?);

    let state = Arc::new(AppState {
        config: config.clone(),
        repo: repo.clone(),
        crawler_tx: Arc::new(tx),
    });

    tokio::spawn(async move {
        let _ = dispatch.run(repo).await;
    });

    let addr = tokio::net::TcpListener::bind(&config.service.net.host).await?;

    let router = axum::Router::new()
        .route("/health", get(handlers::basic::health))
        .route(
            "/tasks", post(handlers::business::add_tasks)
            .patch(handlers::business::patch_tasks)
        )
        .route("/literatures/{id}", patch(handlers::business::patch_literatures))
        .route("/tags/{id}", patch(handlers::business::patch_tags))
        .route("/metadata/{id}", patch(handlers::business::patch_metadata))
        .route("/tokens/{id}", patch(handlers::business::patch_tokens))
        .route("/manga/{id}", get(handlers::business::select_full_data_by_id))
        .layer(from_fn_with_state(state.clone(), middleware::authorization))
        .with_state(state.clone());

    let app = axum::serve(addr, router).await?;

    Ok(app)
}
