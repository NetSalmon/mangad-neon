use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

pub mod crawler;

#[derive(Serialize, Debug)]
pub struct ResponseBody<T> {
    pub ok: bool,
    pub result: T,
}

pub struct ApiResponse<T>(pub T);

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        let body = ResponseBody {
            ok: true,
            result: self.0,
        };

        (StatusCode::OK, Json(body)).into_response()
    }
}

impl<T> ApiResponse<T> {
    pub fn from(n: T) -> ApiResponse<T> {
        ApiResponse(n)
    }
}
