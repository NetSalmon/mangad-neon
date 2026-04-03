use crate::core::entities::dao::ApiResponse;
use crate::error::Error;

pub type ApiResult<T> = Result<ApiResponse<T>, Error>;

pub mod basic {
    use crate::core::entities::dao::ApiResponse;
    use crate::core::service::handlers::ApiResult;

    pub async fn health() -> ApiResult<String> {
        Ok(ApiResponse::from("service is running".into()))
    }
}

pub mod business {
    use std::sync::Arc;
    use axum::extract::State;
    use axum::Json;
    use tokio::sync::oneshot;
    use crate::core::entities::dao::ApiResponse;
    use crate::core::entities::dao::crawler::Task;
    use crate::core::entities::inner::InnerTask;
    use crate::core::service::AppState;
    use crate::core::service::handlers::ApiResult;

    pub async fn add_tasks(
        State(state): State<Arc<AppState>>,
        Json(task): Json<Task>
    ) -> ApiResult<i32> {
        let (tx, rx) = oneshot::channel();
        let inner_task = InnerTask {
            task,
            id_tx: tx,
        };
        
        state.crawler_tx.send(inner_task).await?;
        
        let id = rx.await?;
        
        Ok(ApiResponse::from(id))
    }
}
