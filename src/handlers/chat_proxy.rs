use axum::{
    body::Body,
    extract::{Path, State},
    http::StatusCode,
    response::Response,
};
use reqwest::Client;

/// Proxy configuration
#[derive(Clone)]
pub struct ProxyConfig {
    pub typescript_backend_url: String,
    pub client: Client,
}

impl ProxyConfig {
    pub fn new(typescript_backend_url: String) -> Self {
        Self {
            typescript_backend_url,
            client: Client::new(),
        }
    }
}

/// POST /api/chat
/// Proxy chat requests to TypeScript backend
pub async fn chat_proxy_handler(
    State(config): State<ProxyConfig>,
    body: String,
) -> Result<Response, StatusCode> {
    let url = format!("{}/api/chat", config.typescript_backend_url);

    // Forward request to TypeScript backend
    let response = config
        .client
        .post(&url)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to proxy chat request: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

    // Check if TypeScript backend returned error
    if !response.status().is_success() {
        tracing::error!("TypeScript backend returned error: {}", response.status());
        return Err(StatusCode::BAD_GATEWAY);
    }

    // Stream response back to client
    let status = response.status();
    let headers = response.headers().clone();
    let body_stream = response.bytes_stream();

    let mut builder = Response::builder().status(status);

    // Copy relevant headers
    for (key, value) in headers.iter() {
        let key_str = key.as_str();
        if key_str != "content-length" && key_str != "transfer-encoding" {
            builder = builder.header(key, value);
        }
    }

    // Build streaming response
    let response = builder
        .body(Body::from_stream(body_stream))
        .map_err(|e| {
            tracing::error!("Failed to build streaming response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(response)
}

/// POST /api/abort/:request_id
/// Proxy abort requests to TypeScript backend
pub async fn abort_proxy_handler(
    State(config): State<ProxyConfig>,
    Path(request_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let url = format!("{}/api/abort/{}", config.typescript_backend_url, request_id);

    let response = config
        .client
        .post(&url)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to proxy abort request: {}", e);
            StatusCode::BAD_GATEWAY
        })?;

    if response.status().is_success() {
        Ok(StatusCode::OK)
    } else {
        tracing::error!("TypeScript backend abort failed: {}", response.status());
        Err(StatusCode::BAD_GATEWAY)
    }
}
