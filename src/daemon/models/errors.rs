use crate::daemon::models::api::ApiResp;
use crate::daemon::models::tasks::CanonicalizeTask;
use crate::daemon::models::tasks::ReturningTask;
use axum::http::StatusCode;
use axum::http::header::ToStrError;
use axum::response::IntoResponse;
use axum::{Json, http};
use mangad_neon::error::Error;
use reqwest::header::{InvalidHeaderName, InvalidHeaderValue};
use sea_orm::DbErr;
use std::num::ParseIntError;
use tokio::sync::{AcquireError, mpsc, oneshot};
use tokio::task::JoinError;

#[derive(thiserror::Error, Debug)]
pub enum DaemonError {
    #[error(transparent)]
    MangadError(#[from] Error),
    #[error("canonical task send error {0}")]
    CanonicalSendError(#[from] mpsc::error::SendError<CanonicalizeTask>),
    #[error("inner task add error {0}")]
    InnerTaskAddError(#[from] mpsc::error::SendError<ReturningTask>),
    #[error("canonical task receive error {0}")]
    CanonicalReceiveError(#[from] oneshot::error::RecvError),
    #[error("image error {0}")]
    ImageError(#[from] image::ImageError),
    #[error("task join error {0}")]
    TaskJoinError(#[from] JoinError),
    #[error("IO error {0}")]
    IOError(#[from] std::io::Error),
    #[error("repository error {0}")]
    DatabaseError(#[from] DbErr),
    #[error("meilisearch error {0}")]
    MeiliSearchError(#[from] meilisearch_sdk::errors::Error),
    #[error("bad request error {0}")]
    BadRequestError(String),
    #[error("toml serialization error {0}")]
    TomlSerializationError(#[from] toml::ser::Error),
    #[error("sqlx core error {0}")]
    SqlxCoreError(#[from] sqlx_core::error::Error),
    #[error("{0}")]
    CustomError(String),
    #[error("http error {0}")]
    HttpError(#[from] http::Error),
    #[error("semaphore closed error {0}")]
    SemaphoreCloseError(#[from] AcquireError),
    #[error("request error {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("invalid header name {0}")]
    InvalidHeaderNameError(#[from] InvalidHeaderName),
    #[error("invalid header value {0}")]
    InvalidHeaderValueError(#[from] InvalidHeaderValue),
    #[error("parse int error {0}")]
    ParseIntError(#[from] ParseIntError),
    #[error("invalid format")]
    InvalidTokenFormatError,
    #[error("header value to string token")]
    HeaderValueToStringError(#[from] ToStrError),
    #[error("url parse error {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("token permission denied")]
    TokenPermissionDenied,
    #[error("config permission denied")]
    ConfigPermissionDenied,
}

impl IntoResponse for DaemonError {
    fn into_response(self) -> axum::response::Response {
        let code = match self {
            DaemonError::MangadError(ref err) => match err {
                Error::NotFound => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = ApiResp {
            ok: false,
            status_code: Some(code),
            result: self.to_string(),
        };

        (code, Json(body)).into_response()
    }
}
