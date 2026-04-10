use crate::service::AppState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

pub async fn authorization(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    println!("uri: {:?}", request.uri());
    println!("uri path: {:?}", request.uri().path());
    println!("headers: {:#?}", request.headers());

    let token = request.headers().get("Authorization");
    let Some(auth) = token else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let Ok(auth) = auth.to_str() else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let Some(token) = auth.strip_prefix("Bearer ") else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let Ok(ok) = state.repo.verify_token(token).await else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    if !ok {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}
