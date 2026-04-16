use crate::daemon::service::AppState;
use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

pub async fn authorization(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !state.config.read().await.service.enable_auth {
        tracing::info!("authorization disabled");
        return Ok(next.run(request).await);
    }

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

    let Ok(ok) = state.worker.repo.verify_token(token).await else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    if !ok {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

pub async fn log(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let start = Instant::now();

    let method = request.method().clone();
    let path = request.uri().path().to_string();

    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| addr.ip().to_string());

    let response = next.run(request).await;

    let latency = start.elapsed();
    let status = response.status();
    let now = Utc::now().format("%d/%b/%Y:%H:%M:%S %z");

    tracing::info!(
        ip = %client_ip,
        timestamp = %now,
        method = %method,
        path = %path,
        status = %status.as_u16(),
        latency = ?latency,
        "request_processed"
    );

    Ok(response)
}
