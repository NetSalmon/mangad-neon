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

pub mod business {}
