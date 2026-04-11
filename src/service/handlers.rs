use mangad_neon::core::entities::dao::ApiResp;
use mangad_neon::error::Error;

pub type ApiResult<T> = Result<ApiResp<T>, Error>;

pub mod basic {
    use crate::service::handlers::ApiResult;

    pub async fn health() -> ApiResult<String> {
        Ok("service is running".into())
    }
}

pub mod business {
    use std::convert::Infallible;
    use crate::service::AppState;
    use crate::service::handlers::ApiResult;
    use axum::Json;
    use axum::extract::{Path, Query, State};
    use mangad_neon::core::entities::dao::active::IntoActiveModel;
    use mangad_neon::core::entities::dao::crawler::Task;
    use mangad_neon::core::entities::dao::{
        Document, FullData, InlineLiterature, InlineTag, SearchQuery, active,
    };
    use mangad_neon::core::entities::inner::InnerTask;
    use mangad_neon::core::entities::orm::{
        literatures, metadata, tag_metadata, tags, tasks, tokens,
    };
    use mangad_neon::error::Error;
    use meilisearch_sdk::search::SearchResult;
    use paste::paste;
    use sea_orm::entity::prelude::*;
    use sea_orm::{Set, TransactionTrait};
    use std::sync::Arc;
    use axum::response::Sse;
    use axum::response::sse::Event;
    use tokio::sync::oneshot;
    use tokio_stream::Stream;

    pub async fn add_tasks(
        State(state): State<Arc<AppState>>,
        Json(task): Json<Task>,
    ) -> ApiResult<i32> {
        let (tid_tx, tid_rx) = oneshot::channel();
        let inner_task = InnerTask { task, tid_tx };

        state.crawler_tx.send(inner_task).await?;

        let id = tid_rx.await?;

        Ok(id.into())
    }

    macro_rules! patch {
        ($name:ident, $path:path, [ $($f:ident : $t:ty),* ]) => {
            paste! {
                pub async fn [<patch_ $name>] (
                    State(state): State<Arc<AppState>>,
                    $(Path($f): Path<$t>,)*
                    Json(data): Json<active::[<$name:camel>]>,
                ) -> ApiResult<$path::Model> {
                    let mut active = data.into_active_model();

                    $(active.$f = Set($f);)*

                    let result = $path::Entity::update(active)
                        .exec(&state.repo.db)
                        .await?;
                    Ok(result.into())
                }
            }
        };
    }

    macro_rules! patch_many {
        ($( $name:ident - [ $($f:ident : $t:ty),* $(,)? ] ),* $(,)?) => {
            $(
                patch!($name, mangad_neon::core::entities::orm::$name, [ $($f : $t),* ]);
            )*
        }
    }

    patch_many!(
        tags - [id:i32],
        tasks - [id:i32],
        metadata - [id:i32],
        tokens - [id:Uuid],
        literatures - [id:i32]
    );

    pub async fn select_tags_by_id(
        State(state): State<Arc<AppState>>,
        Path(id): Path<i32>,
    ) -> ApiResult<Option<tags::Model>> {
        Ok(tags::Entity::find_by_id(id)
            .one(&state.repo.db)
            .await?
            .into())
    }

    pub async fn select_full_data_by_id(
        State(state): State<Arc<AppState>>,
        Path(id): Path<i32>,
    ) -> ApiResult<FullData> {
        let tx = state.repo.db.begin().await?;

        let m = metadata::Entity::find_by_id(id)
            .one(&tx)
            .await?
            .ok_or(Error::NotFound)?;

        let mt = tag_metadata::Entity::find()
            .filter(tag_metadata::Column::MetadataId.eq(id))
            .all(&tx)
            .await?
            .iter()
            .map(|tag| tag.tag_id)
            .collect::<Vec<i32>>();

        let tags = tags::Entity::find()
            .filter(tags::Column::Id.is_in(mt))
            .all(&tx)
            .await?
            .into_iter()
            .map(|t| t.into())
            .collect::<Vec<InlineTag>>();

        let literatures = literatures::Entity::find()
            .filter(literatures::Column::MetadataId.eq(id))
            .all(&tx)
            .await?
            .into_iter()
            .map(|t| t.into())
            .collect::<Vec<InlineLiterature>>();

        tx.commit().await?;

        let fin = FullData {
            id: m.id,
            page_count: m.page_count,
            upload: m.upload,
            literatures,
            tags,
        };

        Ok(fin.into())
    }

    macro_rules! select {
        ($entity:ident - $($key:ident : $t:ident),*$(,)?) => {
            paste! {
                pub async fn [<select_ $entity>](
                    State(state): State<Arc<AppState>>,
                    $(Path($key): Path<$t>,)*
                ) -> ApiResult<$entity::Model> {
                    let result = $entity::Entity::find()
                        $(
                            .filter($entity::Column::[<$key:camel>].eq($key))
                        )*
                        .one(&state.repo.db)
                        .await?
                        .ok_or(Error::NotFound)?;
                    Ok(result.into())
                }
            }
        };
    }

    select!(tags - id:i32);
    select!(tag_metadata - tag_id:i32, metadata_id:i32);
    select!(metadata - id:i32);
    select!(tasks - id:i32);
    select!(literatures - id:i32);
    select!(tokens - id:Uuid);

    macro_rules! delete {
        ($entity:ident - $($key:ident : $t:ident),*$(,)?) => {
            paste! {
                pub async fn [<delete_ $entity>](
                    State(state): State<Arc<AppState>>,
                    $(Path($key): Path<$t>,)*
                ) -> ApiResult<$entity::Model> {
                    let active = $entity::ActiveModel {
                        $($key: Set($key),)*
                        ..Default::default()
                    };

                    let r = $entity::Entity::delete(active)
                        .exec_with_returning(&state.repo.db)
                        .await?
                        .ok_or(Error::NotFound)?;

                    Ok(r.into())
                }
            }
        };
    }

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
                    .index
                    .search()
                    .with_query(&q)
                    .execute::<Document>()
                    .await?
                    .hits
            }
            (None, Some(f)) => {
                state
                    .index
                    .search()
                    .with_filter(&f)
                    .execute::<Document>()
                    .await?
                    .hits
            }
            (None, None) => {
                return Err(Error::BadRequestError("need query".to_string()));
            }
        };

        Ok(r.into())
    }

    pub async fn task_notice(
        State(state): State<Arc<AppState>>,
    ) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, Error> {
        let mut task_rx = state.task_tx.subscribe();
        let mut sub_task_rx = state.sub_task_tx.subscribe();

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
}

pub mod resource {
    use crate::service::AppState;
    use axum::body::Body;
    use axum::extract::{Path, State};
    use axum::response::Response;
    use mangad_neon::error::Error;
    use std::sync::Arc;
    use tokio::fs::File;

    pub async fn images(
        State(state): State<Arc<AppState>>,
        Path((mid, index)): Path<(i32, i32)>,
    ) -> Result<Response<Body>, Error> {
        let dir = format!("{:0>10}", mid);
        let file = format!("{:0>10}.webp", index);

        let path = state.config.crawler.storage.join(dir).join(file);

        println!("{}", path.display());

        let f = File::open(path).await?;
        let stream = tokio_util::io::ReaderStream::new(f);
        let body = Body::from_stream(stream);
        let response = Response::builder()
            .header(axum::http::header::CONTENT_TYPE, "image/webp")
            .body(body)?;

        Ok(response)
    }
}
