use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct ApiResp<T> {
    pub ok: bool,
    #[serde(skip_serializing)]
    pub status_code: Option<StatusCode>,
    pub result: T,
}

impl<T: Serialize> IntoResponse for ApiResp<T> {
    fn into_response(self) -> Response {
        let code = self.status_code.unwrap_or(StatusCode::OK);
        (code, Json(self)).into_response()
    }
}

impl From<&str> for ApiResp<String> {
    fn from(t: &str) -> ApiResp<String> {
        ApiResp {
            ok: true,
            status_code: None,
            result: t.to_string(),
        }
    }
}

impl<T: Serialize> From<T> for ApiResp<T> {
    fn from(t: T) -> ApiResp<T> {
        ApiResp {
            ok: true,
            status_code: None,
            result: t,
        }
    }
}

impl<T: Serialize> From<(StatusCode, T)> for ApiResp<T> {
    fn from(t: (StatusCode, T)) -> ApiResp<T> {
        let is_ok = !t.0.is_client_error() && !t.0.is_server_error();
        ApiResp {
            ok: is_ok,
            status_code: Some(t.0),
            result: t.1,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchQuery {
    pub query: Option<String>,
    pub filter: Option<String>,
}
