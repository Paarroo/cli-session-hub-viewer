use axum::{
    extract::Path,
    http::StatusCode,
    Json,
};
use serde::Serialize;

use crate::history::grouping::ConversationSummary;
use crate::history::list_project_summaries;

#[derive(Debug, Serialize)]
pub struct HistoryListResponse {
    pub conversations: Vec<ConversationSummary>,
}

#[derive(Debug, Serialize)]
pub struct ActiveSessionResponse {
    pub session_id: Option<String>,
    pub project_name: String,
    pub updated_at: Option<String>,
}

/// GET /api/projects/:encoded_name/histories
/// List all conversation summaries for a project (like claude-code-webui)
pub async fn list_histories_handler(
    Path(encoded_name): Path<String>,
) -> Result<Json<HistoryListResponse>, StatusCode> {
    let summaries = list_project_summaries(&encoded_name)
        .map_err(|e| {
            tracing::error!("Failed to list summaries for {}: {}", encoded_name, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = HistoryListResponse { conversations: summaries };

    Ok(Json(response))
}

/// GET /api/projects/:encoded_name/active-session
/// Get the most recently modified session for a project (to resume it)
pub async fn get_active_session_handler(
    Path(encoded_name): Path<String>,
) -> Result<Json<ActiveSessionResponse>, StatusCode> {
    let summaries = list_project_summaries(&encoded_name)
        .map_err(|e| {
            tracing::error!("Failed to list summaries for {}: {}", encoded_name, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Find the most recently updated session (excluding agent- files)
    let active_session = summaries
        .into_iter()
        .filter(|s| !s.session_id.starts_with("agent-"))
        .max_by(|a, b| a.last_time.cmp(&b.last_time));

    let response = ActiveSessionResponse {
        session_id: active_session.as_ref().map(|s| s.session_id.clone()),
        project_name: encoded_name,
        updated_at: active_session.map(|s| s.last_time.clone()),
    };

    Ok(Json(response))
}
