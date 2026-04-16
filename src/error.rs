use sea_orm::DbErr;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error {0}")]
    IOError(#[from] std::io::Error),
    #[error("config parse error {0}")]
    ConfigParseError(#[from] toml::de::Error),
    #[error("json error {0}")]
    JsonError(#[from] serde_json::error::Error),
    #[error("repository error {0}")]
    DatabaseError(#[from] DbErr),
    #[error("repository not founded error {0}")]
    DatabaseNotFoundedError(String),
    #[error("token hash error {0}")]
    TokenHashError(String),
    #[error("uuid error {0}")]
    UuidError(#[from] uuid::Error),
    #[error("invalid format")]
    InvalidTokenFormatError,
    #[error("not found error")]
    NotFound,
    #[error("sqlx core error {0}")]
    SqlxCoreError(#[from] sqlx_core::error::Error),
}
