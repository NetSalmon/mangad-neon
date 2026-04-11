pub mod handlers;
pub mod middleware;

use crate::crawler::Dispatch;
use crate::searching::sync;
use crate::thumbnail::ThumbnailTaskSingle;
use crate::{searching, thumbnail};
use axum::middleware::{from_fn, from_fn_with_state};
use axum::routing::{get, patch, post};
use mangad_neon::core::config::Config;
use mangad_neon::core::entities::dao::SubTaskResult;
use mangad_neon::core::entities::inner::InnerTask;
use mangad_neon::core::entities::orm::tasks;
use mangad_neon::core::repository::{IntoDatabaseUrl, Repository};
use mangad_neon::error::Error;
use meilisearch_sdk::indexes::Index;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub repo: Arc<Repository>,
    pub index: Arc<Index>,
    pub crawler_tx: Arc<tokio::sync::mpsc::Sender<InnerTask>>,
    pub config_path: Arc<PathBuf>,
    pub thumbnail_single_tx: tokio::sync::mpsc::Sender<ThumbnailTaskSingle>,
    pub task_tx: tokio::sync::broadcast::Sender<tasks::Model>,
    pub _task_rx: tokio::sync::broadcast::Receiver<tasks::Model>,
    pub sub_task_tx: tokio::sync::broadcast::Sender<SubTaskResult>,
    pub _sub_task_rx: tokio::sync::broadcast::Receiver<SubTaskResult>,
}

pub async fn service(config: Config, path: PathBuf) -> Result<(), Error> {
    tracing::info!("Starting service on {}", config.service.net.host);
    let wrapping_config = Arc::new(config.clone());

    let (mut dispatch, tx) = Dispatch::new(wrapping_config.clone());
    let (sub_task_tx, sub_task_rx) = tokio::sync::broadcast::channel::<SubTaskResult>(1024);
    let (thumbnail_tx, thumbnail_rx) = tokio::sync::mpsc::channel(1024);
    let (thumbnail_single_tx, thumbnail_single_rx) = tokio::sync::mpsc::channel(1024);

    let repo = Arc::new(Repository::new(&wrapping_config.to_database_url()).await?);
    let index = Arc::new(searching::index(wrapping_config.clone()).await?);

    tracing::info!("Database and Search index connected");

    let (task_tx, task_rx) = tokio::sync::broadcast::channel::<tasks::Model>(1024);

    let state = Arc::new(AppState {
        config: Arc::new(RwLock::new(config)),
        index: index.clone(),
        repo: repo.clone(),
        crawler_tx: Arc::new(tx),
        config_path: Arc::new(path),
        thumbnail_single_tx,
        task_tx: task_tx.clone(),
        _task_rx: task_rx,
        sub_task_tx: sub_task_tx.clone(),
        _sub_task_rx: sub_task_rx,
    });

    let clone_repo = repo.clone();

    let clone_config = wrapping_config.clone();
    tokio::spawn(async move {
        if let Err(err) = thumbnail::thumbnail(clone_config, thumbnail_rx).await {
            tracing::error!("Thumbnail encode error: {}", err);
        }
    });

    let clone_config = wrapping_config.clone();
    tokio::spawn(async move {
        if let Err(err) = thumbnail::thumbnail_single(clone_config, thumbnail_single_rx).await {
            tracing::error!("Thumbnail encode error: {}", err);
        }
    });

    tokio::spawn(async move {
        if let Err(err) = dispatch
            .run(clone_repo, task_tx, sub_task_tx, thumbnail_tx)
            .await
        {
            tracing::error!("Crawler dispatch error: {:?}", err);
        }
    });

    tokio::spawn(async move {
        if let Err(err) = sync(repo.clone(), index).await {
            tracing::error!("Search sync error: {:?}", err);
        }
    });

    let addr = tokio::net::TcpListener::bind(&wrapping_config.service.net.host).await?;

    let public_routes = axum::Router::new()
        .route("/health", get(handlers::basic::health))
        .route("/tasks/{id}", get(handlers::business::select_tasks))
        .route("/tasks", get(handlers::business::task_notice))
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
        .route("/tokens/{id}", patch(handlers::business::patch_tokens))
        .route(
            "/config",
            get(handlers::configure::select_config).post(handlers::configure::update_config),
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
