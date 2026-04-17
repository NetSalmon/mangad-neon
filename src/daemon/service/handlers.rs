use crate::daemon::models::api::ApiResp;
use crate::daemon::models::errors::DaemonError;
use serde::Deserialize;

/// API响应的结果类型，包含成功的数据或错误。
pub type ApiResult<T> = Result<ApiResp<T>, DaemonError>;
/// 分页查询参数结构体。
#[derive(Debug, Deserialize)]
pub struct PagedQuery {
    /// 每页大小，默认50。
    #[serde(default = "default_page_size")]
    size: u64,
    /// 偏移量，默认0。
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

    /// 后台任务的运行状态枚举。
    #[derive(Serialize, Debug, Clone)]
    #[serde(rename_all = "lowercase")]
    pub enum SpawnStatus {
        /// 正在运行。
        Running,
        /// 未知状态。
        Unknown,
        /// 错误状态，包含错误消息。
        Error { message: String },
    }

    impl Default for SpawnStatus {
        fn default() -> Self {
            Self::Unknown
        }
    }

    /// 系统状态结构体，包含各个后台服务的状态。
    #[derive(Serialize, Debug, Default)]
    pub struct SystemStatus {
        /// 缩略图服务的状态。
        thumbnail: SpawnStatus,
        /// 分发服务的状态。
        dispatch: SpawnStatus,
        /// 同步服务的状态。
        sync: SpawnStatus,
        /// 规范化服务的状态。
        canonicalize: SpawnStatus,
    }

    /// 获取系统健康状态。
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
    use mangad_neon::db::models::{FullData, InlineTag};
    use mangad_neon::db::models::Task;
    use mangad_neon::db::entities::{literatures, metadata, tag_metadata, tags, tasks};
    use mangad_neon::db::repository;
    use mangad_neon::error::Error;
    use meilisearch_sdk::search::SearchResult;
    use sea_orm::entity::prelude::*;
    use sea_orm::{Set, TransactionTrait};
    use std::convert::Infallible;
    use std::sync::Arc;
    use tokio::sync::oneshot;
    use tokio_stream::Stream;

    /// 添加任务
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

    /// 查找具体漫画数据
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
    select!(metadata - id:i32);
    select!(tasks - id:i32);
    select!(literatures - id:i32);

    delete!(tags - id:i32);
    delete!(tag_metadata - tag_id:i32, metadata_id:i32);
    delete!(metadata - id:i32);
    delete!(literatures - id:i32);

    /// 漫画搜索，过滤器可用字段
    ///
    /// | 字段名      | 描述         |
    /// | ---------- | ----------- |
    /// | genre      | 具体标签      |
    /// | artists    | 漫画作者      |
    /// | origins    | 漫画设计的原作 |
    /// | serials    | 漫画系列      |
    /// | characters | 漫画设计角色   |
    /// | groups     | 漫画创作团体   |
    /// | languages  | 漫画语言      |
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

    /// 任务通知，SSE流
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

    paged_select!(tags);
    paged_select!(metadata);
    paged_select!(tasks);
    paged_select!(literatures);

    /// 分页查询全部漫画数据
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

    /// 查询漫画全部标签
    pub async fn select_manga_tags(
        State(state): State<Arc<AppState>>,
        Path(mid): Path<i32>,
    ) -> ApiResult<Vec<InlineTag>> {
        let (tid, weights) = tag_metadata::Entity::find()
            .filter(tag_metadata::Column::MetadataId.eq(mid))
            .all(&state.worker.repo.db)
            .await?
            .into_iter()
            .map(|m| (m.tag_id, m.weight))
            .collect::<(Vec<i32>, Vec<i32>)>();

        let resp = tags::Entity::find()
            .filter(tags::Column::Id.is_in(tid))
            .all(&state.worker.repo.db)
            .await?
            .into_iter()
            .zip(weights)
            .map(|(m, weight)| (m, weight).into())
            .collect::<Vec<InlineTag>>();

        Ok(resp.into())
    }

    /// 查询漫画单个标签数据
    pub async fn select_manga_tag(
        State(state): State<Arc<AppState>>,
        Path((mid, tid)): Path<(i32, i32)>,
    ) -> ApiResult<InlineTag> {
        let weight = tag_metadata::Entity::find_by_id((mid, tid))
            .one(&state.worker.repo.db)
            .await?
            .map(|m| m.weight)
            .ok_or(DaemonError::NotFound)?;

        let resp = tags::Entity::find_by_id(tid).one(&state.worker.repo.db).await?.ok_or(DaemonError::NotFound)?;
        let tag: InlineTag = (resp, weight).into();
        Ok(tag.into())
    }

    /// 给漫画添加标签
    pub async fn insert_manga_tag(
        State(state): State<Arc<AppState>>,
        Path(mid): Path<i32>,
        Json(body): Json<InlineTag>
    ) -> ApiResult<tags::Model> {
        let InlineTag{ r#type, label, weight, .. } = body;

        let model = tags::Entity::find()
            .filter(
                tags::Column::Type.eq(r#type.clone())
                    .and(tags::Column::Label.eq(label.clone()))
            )
            .one(&state.worker.repo.db)
            .await?;

        let (id, model): (i32, tags::Model) = if let Some(model) = model {
            (model.id, model)
        } else {
            let active = tags::ActiveModel {
                r#type: Set(r#type),
                label: Set(label),
                ..Default::default()
            };

            let res = tags::Entity::insert(active)
                .exec_with_returning(&state.worker.repo.db)
                .await?;

            (res.id, res)
        };

        let active = tag_metadata::ActiveModel {
            metadata_id: Set(mid),
            tag_id: Set(id),
            weight: Set(weight),
        };

        tag_metadata::Entity::insert(active)
            .exec(&state.worker.repo.db)
            .await?;

        Ok(model.into())
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

    /// 获取漫画对应图片文件
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

    /// 获取漫画对应图片缩略图文件
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

    /// 获取服务配置
    pub async fn select_config(State(state): State<Arc<AppState>>) -> ApiResult<Config> {
        let config = state.config.read().await.clone();
        if !config.permissions.allow_config_remote_read {
            Err(DaemonError::ConfigPermissionDenied)?;
        }
        Ok(config.into())
    }

    /// 更新服务配置
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

    /// 获取全部令牌
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

    /// 创建令牌的请求体结构体。
    #[derive(Deserialize)]
    pub struct CreateTokenBody {
        /// 令牌过期时间，默认值。
        #[serde(default)]
        pub expire_time: ExpireTime,
        /// 备注。
        pub remark: Option<String>,
        /// 描述。
        pub description: Option<String>,
    }

    /// 创建新的令牌
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

    /// 吊销令牌的请求体枚举。
    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[serde(tag = "type")]
    pub enum RevokeBody {
        /// 通过UUID吊销。
        Uuid { data: Uuid },
        /// 通过令牌字符串吊销。
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

    /// 吊销具体令牌
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

    /// 根据UUID查找令牌。
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

    /// 更新令牌信息。
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
