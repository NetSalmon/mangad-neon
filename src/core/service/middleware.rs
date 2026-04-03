use crate::core::service::AppState;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

pub async fn authorization(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // testing
    println!("uri: {:?}", request.uri());
    println!("uri path: {:?}", request.uri().path());
    println!("headers: {:#?}", request.headers());

    Ok(next.run(request).await)
}
