//! Conversation management handlers
//!
//! Handles conversation operations including retrieval, deletion from both
//! SurrealDB and Claude's JSONL files.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::history::{load_conversation, ContentBlock};
use crate::domain::models::{Conversation, Message};
use chrono::{DateTime, Utc};
use crate::infrastructure::database::{Database, ConversationRepository};

/// State for conversation handlers
#[derive(Clone)]
pub struct ConversationHandlerState {
    pub db: Database,
}

/// Response for delete operations
#[derive(Serialize)]
pub struct DeleteConversationResponse {
    pub success: bool,
    pub message: String,
    pub session_id: String,
    pub files_deleted: Vec<String>,
    pub files_failed: Vec<String>,
}

/// DELETE /api/conversations/:session_id
/// Hard delete a conversation from DB and optionally from JSONL files
pub async fn delete_conversation_handler(
    State(state): State<ConversationHandlerState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    tracing::info!(session_id = %session_id, "Deleting conversation");

    // Delete from database and get source files
    match ConversationRepository::hard_delete(&state.db, &session_id).await {
        Ok(source_files) => {
            let mut files_deleted = Vec::new();
            let mut files_failed = Vec::new();

            // Delete the JSONL files
            for file_path in source_files {
                let path = PathBuf::from(&file_path);
                if path.exists() {
                    match std::fs::remove_file(&path) {
                        Ok(_) => {
                            tracing::info!(path = %file_path, "Deleted JSONL file");
                            files_deleted.push(file_path);
                        }
                        Err(e) => {
                            tracing::warn!(path = %file_path, "Failed to delete file: {}", e);
                            files_failed.push(format!("{}: {}", file_path, e));
                        }
                    }
                } else {
                    tracing::debug!(path = %file_path, "File not found, skipping");
                }
            }

            (
                StatusCode::OK,
                Json(DeleteConversationResponse {
                    success: true,
                    message: "Conversation deleted successfully".to_string(),
                    session_id,
                    files_deleted,
                    files_failed,
                }),
            )
        }
        Err(e) => {
            tracing::error!(session_id = %session_id, "Failed to delete: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(DeleteConversationResponse {
                    success: false,
                    message: format!("Failed to delete conversation: {}", e),
                    session_id,
                    files_deleted: Vec::new(),
                    files_failed: Vec::new(),
                }),
            )
        }
    }
}

/// Response for soft delete
#[derive(Serialize)]
pub struct SoftDeleteResponse {
    pub success: bool,
    pub message: String,
    pub session_id: String,
}

/// POST /api/conversations/:session_id/archive
/// Soft delete (archive) a conversation - keeps it in DB but hidden
pub async fn archive_conversation_handler(
    State(state): State<ConversationHandlerState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    tracing::info!(session_id = %session_id, "Archiving conversation");

    match ConversationRepository::soft_delete(&state.db, &session_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(SoftDeleteResponse {
                success: true,
                message: "Conversation archived".to_string(),
                session_id,
            }),
        ),
        Err(e) => {
            tracing::error!(session_id = %session_id, "Failed to archive: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(SoftDeleteResponse {
                    success: false,
                    message: format!("Failed to archive: {}", e),
                    session_id,
                }),
            )
        }
    }
}

/// Request body for updating conversation metadata
#[derive(Deserialize)]
pub struct UpdateConversationRequest {
    pub title: Option<String>,
    pub is_favorite: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

/// Response for update operations
#[derive(Serialize)]
pub struct UpdateConversationResponse {
    pub success: bool,
    pub message: String,
}

/// PATCH /api/conversations/:session_id
/// Update conversation metadata (title, favorite, tags, notes)
pub async fn update_conversation_handler(
    State(state): State<ConversationHandlerState>,
    Path(session_id): Path<String>,
    Json(request): Json<UpdateConversationRequest>,
) -> impl IntoResponse {
    tracing::info!(session_id = %session_id, "Updating conversation metadata");

    match ConversationRepository::update_metadata(
        &state.db,
        &session_id,
        request.title,
        request.is_favorite,
        request.tags,
        request.notes,
    )
    .await
    {
        Ok(_) => (
            StatusCode::OK,
            Json(UpdateConversationResponse {
                success: true,
                message: "Conversation updated".to_string(),
            }),
        ),
        Err(e) => {
            tracing::error!(session_id = %session_id, "Failed to update: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(UpdateConversationResponse {
                    success: false,
                    message: format!("Failed to update: {}", e),
                }),
            )
        }
    }
}

/// Path parameters for GET conversation
#[derive(Deserialize)]
pub struct GetConversationPath {
    pub encoded_name: String,
    pub session_id: String,
}

/// GET /api/projects/{encoded_name}/histories/{session_id}
/// Retrieve full conversation history with all messages
/// Returns domain::models::Conversation format for frontend compatibility
pub async fn get_conversation_handler(
    Path(params): Path<GetConversationPath>,
) -> Result<Json<Conversation>, (StatusCode, String)> {
    tracing::info!(
        encoded_name = %params.encoded_name,
        session_id = %params.session_id,
        "Fetching conversation"
    );

    match load_conversation(&params.encoded_name, &params.session_id) {
        Ok(history) => {
            tracing::info!(
                session_id = %params.session_id,
                messages_count = history.messages.len(),
                "Conversation loaded successfully"
            );

            // Convert ConversationHistory to domain::models::Conversation
            let messages: Vec<Message> = history
                .messages
                .into_iter()
                .map(|m| {
                    let timestamp = m
                        .timestamp
                        .as_ref()
                        .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now);

                    let content_text = m
                        .content
                        .iter()
                        .filter_map(|c| match c {
                            ContentBlock::Text { text } => Some(text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    match m.role.as_str() {
                        "user" | "human" => Message::User {
                            content: content_text,
                            timestamp,
                            images: vec![],
                            metadata: None,
                        },
                        "assistant" => Message::Assistant {
                            content: content_text,
                            timestamp,
                            model: None,
                            images: vec![],
                            metadata: None,
                        },
                        _ => Message::System {
                            content: content_text,
                            timestamp,
                            level: None,
                            metadata: None,
                        },
                    }
                })
                .collect();

            let conversation = Conversation {
                session_id: history.session_id,
                messages,
            };

            Ok(Json(conversation))
        }
        Err(e) => {
            tracing::error!(
                session_id = %params.session_id,
                "Failed to load conversation: {}", e
            );

            // Distinguish between not found and other errors
            if e.contains("not found") || e.contains("No such file") {
                Err((StatusCode::NOT_FOUND, format!("Conversation not found: {}", e)))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load conversation: {}", e)))
            }
        }
    }
}
