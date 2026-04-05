use crate::core::entities::dao::ApiResp;
use crate::error::Error;

pub type ApiResult<T> = Result<ApiResp<T>, Error>;

pub mod basic {
    use crate::core::service::handlers::ApiResult;

    pub async fn health() -> ApiResult<String> {
        Ok("service is running".into())
    }
}

pub mod business {
    use crate::core::entities::dao::active::IntoActiveModel;
    use crate::core::entities::dao::crawler::Task;
    use crate::core::entities::dao::{active, FullData, InlineLiterature, InlineTag};
    use crate::core::entities::inner::InnerTask;
    use crate::core::entities::orm::{literatures, metadata, tag_metadata, tags};
    use crate::core::service::handlers::ApiResult;
    use crate::core::service::AppState;
    use crate::error::Error;
    use axum::extract::{Path, State};
    use axum::Json;
    use paste::paste;
    use sea_orm::entity::prelude::*;
    use sea_orm::{Set, TransactionTrait};
    use std::sync::Arc;
    use tokio::sync::oneshot;

    pub async fn add_tasks(
        State(state): State<Arc<AppState>>,
        Json(task): Json<Task>,
    ) -> ApiResult<i32> {
        let (tx, rx) = oneshot::channel();
        let inner_task = InnerTask { task, id_tx: tx };

        state.crawler_tx.send(inner_task).await?;

        let id = rx.await?;

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
                    // 假设 id 字段也需要手动 Set，或者已经在 data 里了
                    // active.id = Set(id); 
                    
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
                patch!($name, crate::core::entities::orm::$name, [ $($f : $t),* ]);
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
        Ok(tags::Entity::find_by_id(id).one(&state.repo.db).await?.into())
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
}
