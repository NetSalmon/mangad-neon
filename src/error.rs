use crate::core::entities::dao::ResponseBody;
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use sea_orm::{DbErr, sqlx};
use std::convert::Infallible;

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
    #[error("database error {0}")]
    DatabaseError(#[from] DbErr),
    #[error("database not founded error {0}")]
    DatabaseNotFoundedError(String),
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
            | Error::MissingHeaderError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = ResponseBody {
            ok: false,
            result: self.to_string(),
        };

        (code, Json(body)).into_response()
    }
}
