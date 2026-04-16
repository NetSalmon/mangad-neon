pub mod handlers;
pub mod middleware;
pub mod worker;

use crate::daemon::models::error::AppError;
use crate::daemon::service::worker::{Worker, WorkerHandler};
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::{get, patch, post};
use mangad_neon::core::config::Config;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub config_path: Arc<PathBuf>,
    pub worker: Arc<WorkerHandler>,
}

pub async fn service(config: Config, path: PathBuf) -> Result<(), AppError> {
    tracing::info!("Starting daemon on {}", config.service.net.host);
    let wrapping_config = Arc::new(config.clone());
    tracing::info!("Database and Search index connected");

    let (worker, handler) = Worker::new(wrapping_config.clone()).await?;

    let state = Arc::new(AppState {
        config: Arc::new(RwLock::new(config)),
        config_path: Arc::new(path),
        worker: Arc::new(handler),
    });

    worker.spawn().await;

    let addr = tokio::net::TcpListener::bind(&wrapping_config.service.net.host).await?;

    let public_routes = axum::Router::new()
        .route("/health", get(handlers::basic::health))
        .route("/tasks/{id}", get(handlers::business::select_tasks))
        .route("/notice", get(handlers::business::task_notice))
        .route("/tasks", get(handlers::business::paged_select_tasks))
        .route("/tags", get(handlers::business::paged_select_tags))
        .route("/metadata", get(handlers::business::paged_select_metadata))
        .route(
            "/literatures",
            get(handlers::business::paged_select_literatures),
        )
        .route("/mangas", get(handlers::business::paged_select_full_data))
        .route(
            "/literatures/{id}",
            get(handlers::business::select_literatures),
        )
        .route("/tags/{id}", get(handlers::business::select_tags))
        .route("/metadata/{id}", get(handlers::business::select_metadata))
        .route(
            "/mangas/{id}",
            get(handlers::business::select_full_data_by_id),
        )
        .route("/search", get(handlers::business::searching))
        .route(
            "/mangas/{mid}/images/{index}",
            get(handlers::resource::images),
        )
        .route(
            "/mangas/{mid}/thumbnails/{index}",
            get(handlers::resource::thumbnails),
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
        .route(
            "/config",
            get(handlers::configure::select_config).post(handlers::configure::update_config),
        )
        .route(
            "/tokens",
            post(handlers::tokens::create_token)
                .get(handlers::tokens::list_tokens)
                .patch(handlers::tokens::revoke_token),
        )
        .route(
            "/tokens/{id}",
            get(handlers::tokens::select_tokens).patch(handlers::tokens::patch_tokens),
        )
        .layer(from_fn_with_state(state.clone(), middleware::authorization));

    let router = public_routes
        .merge(private_routes)
        .layer(from_fn(middleware::log))
        .with_state(state.clone());

    let app = axum::serve(
        addr,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(app)
}
