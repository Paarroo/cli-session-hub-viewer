//! Abort handler for canceling running chat requests
//!
//! This handler allows clients to abort ongoing Claude CLI processes.

use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Serialize;

use crate::cli::claude::session_manager::get_session_manager;

/// Response for abort requests
#[derive(Serialize)]
pub struct AbortResponse {
    pub success: bool,
    pub message: String,
    pub request_id: String,
}

/// POST /api/abort/:request_id
/// Abort a running chat request
pub async fn abort_handler(
    Path(request_id): Path<String>,
) -> impl IntoResponse {
    let session_manager = get_session_manager();

    tracing::info!(request_id = %request_id, "Received abort request");

    match session_manager.abort_process(&request_id).await {
        Ok(()) => {
            tracing::info!(request_id = %request_id, "Successfully aborted process");
            (
                StatusCode::OK,
                Json(AbortResponse {
                    success: true,
                    message: "Process aborted successfully".to_string(),
                    request_id,
                }),
            )
        }
        Err(e) => {
            tracing::warn!(request_id = %request_id, "Failed to abort: {}", e);
            (
                StatusCode::NOT_FOUND,
                Json(AbortResponse {
                    success: false,
                    message: format!("Failed to abort: {}", e),
                    request_id,
                }),
            )
        }
    }
}

/// GET /api/processes/active
/// Get count of active processes
#[derive(Serialize)]
pub struct ActiveProcessesResponse {
    pub count: usize,
}

pub async fn active_processes_handler() -> Json<ActiveProcessesResponse> {
    let session_manager = get_session_manager();
    let count = session_manager.active_process_count().await;

    Json(ActiveProcessesResponse { count })
}

/// DELETE /api/sessions/:session_id
/// Remove a session
#[derive(Serialize)]
pub struct SessionDeleteResponse {
    pub success: bool,
    pub message: String,
}

pub async fn delete_session_handler(
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let session_manager = get_session_manager();

    match session_manager.remove_session(&session_id).await {
        Some(_) => {
            tracing::info!(session_id = %session_id, "Session removed");
            (
                StatusCode::OK,
                Json(SessionDeleteResponse {
                    success: true,
                    message: "Session removed successfully".to_string(),
                }),
            )
        }
        None => {
            tracing::warn!(session_id = %session_id, "Session not found");
            (
                StatusCode::NOT_FOUND,
                Json(SessionDeleteResponse {
                    success: false,
                    message: "Session not found".to_string(),
                }),
            )
        }
    }
}
