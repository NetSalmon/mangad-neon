use crate::daemon::models::api::ApiResp;
use crate::daemon::models::errors::DaemonError;
use serde::Deserialize;

pub type ApiResult<T> = Result<ApiResp<T>, DaemonError>;
#[derive(Debug, Deserialize)]
pub struct PagedQuery {
    #[serde(default = "default_page_size")]
    size: u64,
    #[serde(default = "default_offset")]
    offset: u64,
}

fn default_page_size() -> u64 {
    50
}
fn default_offset() -> u64 {
    0
}

macro_rules! delete {
    ($entity:ident - $($key:ident : $t:ident),*$(,)?) => {
        paste::paste! {
            pub async fn [<delete_ $entity>](
                State(state): State<Arc<AppState>>,
                $(Path($key): Path<$t>,)*
            ) -> ApiResult<$entity::Model> {
                let active = $entity::ActiveModel {
                    $($key: Set($key),)*
                    ..Default::default()
                };

                let r = $entity::Entity::delete(active)
                    .exec_with_returning(&state.worker.repo.db)
                    .await?
                    .ok_or(Error::NotFound)?;

                Ok(r.into())
            }
        }
    };
}
macro_rules! patch {
    ($name:ident - $($f:ident : $t:ty),* $(,)?) => {
        paste::paste! {
            pub async fn [<patch_ $name>] (
                State(state): State<Arc<AppState>>,
                $(Path($f): Path<$t>,)*
                Json(data): Json<active::[<$name:camel>]>,
            ) -> ApiResult<$name::Model> {
                let mut active = data.into_active_model();

                $(active.$f = Set($f);)*

                let result = $name::Entity::update(active)
                    .exec(&state.worker.repo.db)
                    .await?;
                Ok(result.into())
            }
        }
    };
}
macro_rules! select {
    ($entity:ident - $($key:ident : $t:ident),*$(,)?) => {
        paste::paste! {
            pub async fn [<select_ $entity>](
                State(state): State<Arc<AppState>>,
                $(Path($key): Path<$t>,)*
            ) -> ApiResult<$entity::Model> {
                let result = $entity::Entity::find()
                    $(
                        .filter($entity::Column::[<$key:camel>].eq($key))
                    )*
                    .one(&state.worker.repo.db)
                    .await?
                    .ok_or(Error::NotFound)?;
                Ok(result.into())
            }
        }
    };
}
macro_rules! paged_select {
    ($entity:ident) => {
        paste::paste! {
            pub async fn [<paged_select_ $entity>](
                State(state): State<Arc<AppState>>,
                Query(query): Query<PagedQuery>,
            ) -> ApiResult<Vec<$entity::Model>> {
                 let resp = state.worker.repo.[<list_ $entity>](query.size, query.offset).await?;
                 Ok(resp.into())
            }
        }
    };
}

pub mod basic {
    use crate::daemon::service::AppState;
    use crate::daemon::service::handlers::ApiResult;
    use axum::extract::State;
    use serde::Serialize;
    use std::sync::Arc;

    #[derive(Serialize, Debug, Clone)]
    #[serde(rename_all = "lowercase")]
    pub enum SpawnStatus {
        Running,
        Unknown,
        Error { message: String },
    }

    impl Default for SpawnStatus {
        fn default() -> Self {
            Self::Unknown
        }
    }

    #[derive(Serialize, Debug, Default)]
    pub struct SystemStatus {
        thumbnail: SpawnStatus,
        dispatch: SpawnStatus,
        sync: SpawnStatus,
        canonicalize: SpawnStatus,
    }

    pub async fn health(State(state): State<Arc<AppState>>) -> ApiResult<SystemStatus> {
        let mut status = SystemStatus::default();
        status.thumbnail = state.worker.watch.thumbnail.borrow().clone();
        status.sync = state.worker.watch.sync.borrow().clone();
        status.dispatch = state.worker.watch.dispatch.borrow().clone();
        status.canonicalize = state.worker.watch.canonicalization.borrow().clone();

        Ok(status.into())
    }
}

pub mod business {
    use crate::daemon::models::active;
    use crate::daemon::models::active::IntoActiveModel;
    use crate::daemon::models::api::SearchQuery;
    use crate::daemon::models::errors::DaemonError;
    use crate::daemon::models::searching::Document;
    use crate::daemon::models::tasks::ReturningTask;
    use crate::daemon::service::AppState;
    use crate::daemon::service::handlers::{ApiResult, PagedQuery};
    use axum::Json;
    use axum::extract::{Path, Query, State};
    use axum::response::Sse;
    use axum::response::sse::Event;
    use mangad_neon::db::models::FullData;
    use mangad_neon::db::models::Task;
    use mangad_neon::db::entities::{literatures, metadata, tag_metadata, tags, tasks, tokens};
    use mangad_neon::db::repository;
    use mangad_neon::error::Error;
    use meilisearch_sdk::search::SearchResult;
    use sea_orm::entity::prelude::*;
    use sea_orm::{Set, TransactionTrait};
    use std::convert::Infallible;
    use std::sync::Arc;
    use tokio::sync::oneshot;
    use tokio_stream::Stream;

    pub async fn add_tasks(
        State(state): State<Arc<AppState>>,
        Json(task): Json<Task>,
    ) -> ApiResult<i32> {
        let (tid_tx, tid_rx) = oneshot::channel();
        let inner_task = ReturningTask { task, tid_tx };

        state.worker.dispatch_tx.send(inner_task).await?;

        let id = tid_rx.await?;

        Ok(id.into())
    }

    patch!(tags - id:i32);
    patch!(tasks - id:i32);
    patch!(metadata - id:i32);
    patch!(literatures - id:i32);

    pub async fn select_tags_by_id(
        State(state): State<Arc<AppState>>,
        Path(id): Path<i32>,
    ) -> ApiResult<Option<tags::Model>> {
        Ok(tags::Entity::find_by_id(id)
            .one(&state.worker.repo.db)
            .await?
            .into())
    }

    pub async fn select_full_data_by_id(
        State(state): State<Arc<AppState>>,
        Path(id): Path<i32>,
    ) -> ApiResult<FullData> {
        let tx = state.worker.repo.db.begin().await?;

        tracing::debug!("starting select full data {}", id);

        let fin = repository::select_full_data_with_tx(id, &tx).await?;

        tracing::debug!("starting select full data {} ok", id);

        tx.commit().await?;

        Ok(fin.into())
    }

    select!(tags - id:i32);
    select!(tag_metadata - tag_id:i32, metadata_id:i32);
    select!(metadata - id:i32);
    select!(tasks - id:i32);
    select!(literatures - id:i32);
    select!(tokens - id:Uuid);

    delete!(tags - id:i32);
    delete!(tag_metadata - tag_id:i32, metadata_id:i32);
    delete!(metadata - id:i32);
    delete!(tasks - id:i32);
    delete!(literatures - id:i32);

    pub async fn searching(
        State(state): State<Arc<AppState>>,
        Query(query): Query<SearchQuery>,
    ) -> ApiResult<Vec<SearchResult<Document>>> {
        let r = match (query.query, query.filter) {
            (Some(q), Some(f)) => {
                state
                    .worker
                    .index
                    .search()
                    .with_query(&q)
                    .with_filter(&f)
                    .execute::<Document>()
                    .await?
                    .hits
            }
            (Some(q), None) => {
                state
                    .worker
                    .index
                    .search()
                    .with_query(&q)
                    .execute::<Document>()
                    .await?
                    .hits
            }
            (None, Some(f)) => {
                state
                    .worker
                    .index
                    .search()
                    .with_filter(&f)
                    .execute::<Document>()
                    .await?
                    .hits
            }
            (None, None) => {
                return Err(DaemonError::BadRequestError("need query".to_string()));
            }
        };

        Ok(r.into())
    }

    pub async fn task_notice(
        State(state): State<Arc<AppState>>,
    ) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, DaemonError> {
        let mut task_rx = state.worker.task_tx.subscribe();
        let mut sub_task_rx = state.worker.sub_task_tx.subscribe();

        let stream = async_stream::stream! {
            loop{
                let (field, data) = tokio::select! {
                    Ok(data) = task_rx.recv() => {("task", serde_json::to_string(&data).unwrap_or(String::new()))}
                    Ok(data) = sub_task_rx.recv() => {("subtask", serde_json::to_string(&data).unwrap_or(String::new()))}
                };
                let event = Event::default().event(field).data(data);
                yield Ok(event);
            }
        };

        Ok(Sse::new(stream).keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("keep-alive"),
        ))
    }

    pub async fn list_tags(
        State(state): State<Arc<AppState>>,
        Query(query): Query<PagedQuery>,
    ) -> ApiResult<Vec<tags::Model>> {
        let resp = state
            .worker
            .repo
            .list_tags(query.size, query.offset)
            .await?;
        Ok(resp.into())
    }

    paged_select!(tags);
    paged_select!(tag_metadata);
    paged_select!(metadata);
    paged_select!(tasks);
    paged_select!(literatures);

    pub async fn paged_select_full_data(
        State(state): State<Arc<AppState>>,
        Query(query): Query<PagedQuery>,
    ) -> ApiResult<Vec<FullData>> {
        let resp = state
            .worker
            .repo
            .list_full_data(query.size, query.offset)
            .await?;
        Ok(resp.into())
    }
}

pub mod resource {
    use crate::daemon::models::errors::DaemonError;
    use crate::daemon::service::AppState;
    use crate::daemon::thumbnail::{TaskType, ThumbnailTask};
    use axum::body::Body;
    use axum::extract::{Path, State};
    use axum::response::Response;
    use std::sync::Arc;
    use tokio::fs::File;

    pub async fn images(
        State(state): State<Arc<AppState>>,
        Path((mid, index)): Path<(i32, i32)>,
    ) -> Result<Response<Body>, DaemonError> {
        let dir = format!("{:0>10}", mid);
        let file = format!("{:0>10}.webp", index);

        let path = state
            .config
            .read()
            .await
            .crawler
            .storage
            .join(dir)
            .join(file);

        tracing::debug!("visit image {}", path.display());

        let f = File::open(path).await?;
        let stream = tokio_util::io::ReaderStream::new(f);
        let body = Body::from_stream(stream);
        let response = Response::builder()
            .header(axum::http::header::CONTENT_TYPE, "image/webp")
            .body(body)?;

        Ok(response)
    }

    pub async fn thumbnails(
        State(state): State<Arc<AppState>>,
        Path((mid, index)): Path<(i32, i32)>,
    ) -> Result<Response<Body>, DaemonError> {
        let dir = format!("{:0>10}", mid);
        let file = format!("{:0>10}.webp", index);

        let path = state
            .config
            .read()
            .await
            .crawler
            .storage
            .join(&dir)
            .join(crate::daemon::THUMBNAIL_PATH)
            .join(&file);

        println!("{}", path.display());

        let f = if let Ok(f) = File::open(path).await {
            f
        } else {
            let path = state
                .config
                .read()
                .await
                .crawler
                .storage
                .join(dir)
                .join(file);
            let _ = state
                .worker
                .thumbnail_tx
                .send(ThumbnailTask {
                    mid,
                    r#type: TaskType::Single(index),
                })
                .await;

            File::open(path).await?
        };

        let stream = tokio_util::io::ReaderStream::new(f);
        let body = Body::from_stream(stream);
        let response = Response::builder()
            .header(axum::http::header::CONTENT_TYPE, "image/webp")
            .body(body)?;

        Ok(response)
    }
}

pub mod configure {
    use crate::daemon::models::errors::DaemonError;
    use crate::daemon::service::AppState;
    use crate::daemon::service::handlers::ApiResult;
    use axum::Json;
    use axum::extract::State;
    use mangad_neon::config::Config;
    use std::sync::Arc;

    pub async fn select_config(State(state): State<Arc<AppState>>) -> ApiResult<Config> {
        let config = state.config.read().await.clone();
        if !config.permissions.allow_config_remote_read {
            Err(DaemonError::ConfigPermissionDenied)?;
        }
        Ok(config.into())
    }

    pub async fn update_config(
        State(state): State<Arc<AppState>>,
        Json(config): Json<Config>,
    ) -> ApiResult<bool> {
        let mut write = state.config.write().await;
        if !write.permissions.allow_config_remote_write {
            Err(DaemonError::ConfigPermissionDenied)?;
        }
        *write = config.clone();
        let content = toml::to_string_pretty(&config)?;
        tokio::fs::write(&*state.config_path, content).await?;
        Ok(true.into())
    }
}

pub mod tokens {
    use crate::daemon::models::active;
    use crate::daemon::models::active::IntoActiveModel;
    use crate::daemon::models::errors::DaemonError;
    use crate::daemon::service::AppState;
    use crate::daemon::service::handlers::{ApiResult, PagedQuery};
    use axum::Json;
    use axum::extract::{Path, Query, State};
    use mangad_neon::db::models::ExpireTime;
    use mangad_neon::db::entities::tokens;
    use mangad_neon::token::TokenTrait;
    use mangad_neon::error::Error;
    use sea_orm::{EntityTrait, Set};
    use serde::Deserialize;
    use std::sync::Arc;
    use uuid::Uuid;

    pub async fn list_tokens(
        State(state): State<Arc<AppState>>,
        Query(paged): Query<PagedQuery>,
    ) -> ApiResult<Vec<tokens::Model>> {
        if !state
            .config
            .read()
            .await
            .permissions
            .allow_token_remote_read
        {
            return Err(DaemonError::TokenPermissionDenied)?;
        }
        let resp = state
            .worker
            .repo
            .list_tokens(paged.size, paged.offset)
            .await?;

        Ok(resp.into())
    }

    #[derive(Deserialize)]
    pub struct CreateTokenBody {
        #[serde(default)]
        pub expire_time: ExpireTime,
        pub remark: Option<String>,
        pub description: Option<String>,
    }

    pub async fn create_token(
        State(state): State<Arc<AppState>>,
        Json(body): Json<CreateTokenBody>,
    ) -> ApiResult<String> {
        if !state
            .config
            .read()
            .await
            .permissions
            .allow_token_remote_create
        {
            return Err(DaemonError::TokenPermissionDenied)?;
        }
        let CreateTokenBody {
            expire_time,
            remark,
            description,
        } = body;
        let (_, b) = state
            .worker
            .repo
            .create_token(expire_time, remark, description)
            .await?;
        Ok(b.into())
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[serde(tag = "type")]
    pub enum RevokeBody {
        Uuid { data: Uuid },
        Token { data: String },
    }

    impl TokenTrait for RevokeBody {
        fn uuid(&self) -> Result<Uuid, Error> {
            match self {
                Self::Uuid { data } => Ok(data.clone()),
                Self::Token { data } => data.uuid(),
            }
        }
    }
    pub async fn revoke_token(
        State(state): State<Arc<AppState>>,
        Json(revoke_body): Json<RevokeBody>,
    ) -> ApiResult<()> {
        if !state
            .config
            .read()
            .await
            .permissions
            .allow_token_remote_revoke
        {
            return Err(DaemonError::TokenPermissionDenied)?;
        }

        state.worker.repo.revoke_token(&revoke_body.uuid()?).await?;

        Ok(().into())
    }

    pub async fn select_tokens(
        State(state): State<Arc<AppState>>,
        Path(id): Path<Uuid>,
    ) -> ApiResult<tokens::Model> {
        let r = tokens::Entity::find_by_id(id)
            .one(&state.worker.repo.db)
            .await?
            .ok_or(Error::NotFound)?;

        Ok(r.into())
    }

    pub async fn patch_tokens(
        State(state): State<Arc<AppState>>,
        Path(id): Path<Uuid>,
        Json(data): Json<active::Tokens>,
    ) -> ApiResult<tokens::Model> {
        if !state
            .config
            .read()
            .await
            .permissions
            .allow_token_remote_modify
        {
            return Err(DaemonError::TokenPermissionDenied)?;
        }
        let mut active = data.into_active_model();

        active.id = Set(id);

        let result = tokens::Entity::update(active)
            .exec(&state.worker.repo.db)
            .await?;
        Ok(result.into())
    }
}
