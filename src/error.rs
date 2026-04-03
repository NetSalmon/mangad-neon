use crate::core::entities::dao::ResponseBody;
use crate::core::entities::inner::{CanonicalizeTask, InnerTask};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, http};
use http::header::ToStrError;
use sea_orm::DbErr;
use std::convert::Infallible;
use tokio::sync::AcquireError;
use tokio::task::JoinError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error {0}")]
    IOError(#[from] std::io::Error),
    #[error("config parse error {0}")]
    ConfigParseError(#[from] toml::de::Error),
    #[error("json error {0}")]
    JsonError(#[from] serde_json::error::Error),
    #[error("missing header error")]
    MissingHeaderError(String),
    #[error("request error {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("testing error {0}")]
    TestingError(String),
    #[error("max retries error {0}")]
    MaxRetriesError(String),
    #[error("image error {0}")]
    ImageError(#[from] image::ImageError),
    #[error("invalid header name {0}")]
    InvalidHeaderNameError(#[from] reqwest::header::InvalidHeaderName),
    #[error("invalid header value {0}")]
    InvalidHeaderValueError(#[from] reqwest::header::InvalidHeaderValue),
    #[error("url parse error {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("infallible {0}")]
    InfallibleError(#[from] Infallible),
    #[error("repository error {0}")]
    DatabaseError(#[from] DbErr),
    #[error("repository not founded error {0}")]
    DatabaseNotFoundedError(String),
    #[error("task join error {0}")]
    TaskJoinError(#[from] JoinError),
    #[error("inner task add error {0}")]
    InnerTaskAddError(#[from] tokio::sync::mpsc::error::SendError<InnerTask>),
    #[error("canonical task send error {0}")]
    CanonicalSendError(#[from] tokio::sync::mpsc::error::SendError<CanonicalizeTask>),
    #[error("canonical task receive error {0}")]
    CanonicalReceiveError(#[from] tokio::sync::oneshot::error::RecvError),
    #[error("semaphore closed error {0}")]
    SemaphoreCloseError(#[from] AcquireError),
    #[error("token hash error {0}")]
    TokenHashError(String),
    #[error("uuid error {0}")]
    UuidError(#[from] uuid::Error),
    #[error("invalid format")]
    InvalidTokenFormatError,
    #[error("header value to string token")]
    HeaderValueToStringError(#[from] ToStrError),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        let code = match self {
            Error::IOError(_)
            | Error::ConfigParseError(_)
            | Error::JsonError(_)
            | Error::RequestError(_)
            | Error::TestingError(_)
            | Error::MaxRetriesError(_)
            | Error::InfallibleError(_)
            | Error::ImageError(_)
            | Error::InvalidHeaderNameError(_)
            | Error::InvalidHeaderValueError(_)
            | Error::UrlParseError(_)
            | Error::DatabaseError(_)
            | Error::DatabaseNotFoundedError(_)
            | Error::TaskJoinError(_)
            | Error::CanonicalSendError(_)
            | Error::CanonicalReceiveError(_)
            | Error::InnerTaskAddError(_)
            | Error::SemaphoreCloseError(_)
            | Error::TokenHashError(_)
            | Error::UuidError(_)
            | Error::InvalidTokenFormatError
            | Error::HeaderValueToStringError(_)
            | Error::MissingHeaderError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = ResponseBody {
            ok: false,
            result: self.to_string(),
        };

        (code, Json(body)).into_response()
    }
}
