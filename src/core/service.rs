pub mod handlers;
pub mod middleware;

use super::entities::config::Config;
use crate::Error;
use crate::core::crawler::Dispatch;
use crate::core::entities::inner::InnerTask;
use crate::core::repository::Repository;
use crate::core::searching;
use crate::core::searching::sync;
use axum::middleware::from_fn_with_state;
use axum::routing::{get, patch, post};
use meilisearch_sdk::indexes::Index;
use std::sync::Arc;

pub struct AppState {
    pub config: Arc<Config>,
    pub repo: Arc<Repository>,
    pub index: Arc<Index>,
    pub crawler_tx: Arc<tokio::sync::mpsc::Sender<InnerTask>>,
}

pub async fn service(config: Arc<Config>) -> Result<(), Error> {
    let (mut dispatch, tx) = Dispatch::new(config.clone());
    let repo = Arc::new(Repository::new(config.clone()).await?);
    let index = Arc::new(searching::index(config.clone()).await?);

    let state = Arc::new(AppState {
        config: config.clone(),
        index: index.clone(),
        repo: repo.clone(),
        crawler_tx: Arc::new(tx),
    });

    let clone_repo = repo.clone();

    tokio::spawn(async move {
        let err = dispatch.run(clone_repo).await;
        println!("dispatch:\n{:#?}", err);
    });

    tokio::spawn(async move {
        let err = sync(repo.clone(), index).await;
        println!("sync:\n{:#?}", err);
    });

    let addr = tokio::net::TcpListener::bind(&config.service.net.host).await?;

    let public_routes = axum::Router::new()
        .route("/health", get(handlers::basic::health))
        .route("/tasks", get(handlers::business::select_tasks))
        .route(
            "/literatures/{id}",
            get(handlers::business::select_literatures),
        )
        .route("/tags/{id}", get(handlers::business::select_tags))
        .route("/metadata/{id}", get(handlers::business::select_metadata))
        .route("/tokens/{id}", get(handlers::business::select_tokens))
        .route(
            "/manga/{id}",
            get(handlers::business::select_full_data_by_id),
        )
        .route("/search", get(handlers::business::searching))
        .route(
            "/mangas/{mid}/images/{index}",
            get(handlers::resource::images),
        );

    let private_routes = axum::Router::new()
        .route(
            "/tasks",
            post(handlers::business::add_tasks).patch(handlers::business::patch_tasks),
        )
        .route(
            "/literatures/{id}",
            patch(handlers::business::patch_literatures)
                .delete(handlers::business::delete_literatures),
        )
        .route(
            "/tags/{id}",
            patch(handlers::business::patch_tags).delete(handlers::business::delete_tags),
        )
        .route(
            "/metadata/{id}",
            patch(handlers::business::patch_metadata).delete(handlers::business::delete_metadata),
        )
        .route("/tokens/{id}", patch(handlers::business::patch_tokens))
        .layer(from_fn_with_state(state.clone(), middleware::authorization));

    let router = public_routes
        .merge(private_routes)
        .with_state(state.clone());

    let app = axum::serve(addr, router).await?;

    Ok(app)
}
