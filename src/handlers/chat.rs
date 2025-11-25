//! Native chat handler using Claude CLI directly
//!
//! This handler spawns the Claude CLI and streams NDJSON responses.

use axum::{
    body::Body,
    extract::Json,
    http::StatusCode,
    response::Response,
    Extension,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::cli::claude::{
    executor::{ClaudeExecutor, ExecuteOptions, PermissionMode, StreamResponse},
    session_manager::{get_session_manager, RequestId},
    detection::{detect_claude_cli, CliDetectionResult},
};
use crate::cli::traits::CliExecutor;

/// Chat request payload (aligned with domain::models::ChatRequest)
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    /// Message/prompt to send
    pub message: String,
    /// Unique request ID for abort functionality (from client)
    #[serde(default)]
    pub request_id: Option<String>,
    /// Session ID for conversation continuity (optional)
    #[serde(default)]
    pub session_id: Option<String>,
    /// Working directory for file operations
    #[serde(default)]
    pub working_directory: Option<String>,
    /// Allowed tools (optional)
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    /// Permission mode (optional)
    #[serde(default)]
    pub permission_mode: Option<String>,
    /// Hide user message in chat (for "continue" commands)
    #[serde(default)]
    pub hide_user_message: Option<bool>,
    /// Image paths for vision capabilities (server filesystem paths)
    #[serde(default)]
    pub image_paths: Vec<String>,
    /// CLI provider to use (claude, opencode, gemini)
    #[serde(default)]
    pub cli_provider: Option<String>,
}

/// Chat handler application state
#[derive(Clone)]
pub struct ChatHandlerState {
    /// Detected CLI information
    pub cli_detection: CliDetectionResult,
}

impl ChatHandlerState {
    pub async fn new() -> Result<Self, String> {
        let cli_detection = detect_claude_cli(None)
            .await
            .map_err(|e| format!("Failed to detect Claude CLI: {}", e))?;

        tracing::info!(
            "Chat handler initialized with CLI at {:?} (version: {})",
            cli_detection.executable_path,
            cli_detection.version
        );

        Ok(Self { cli_detection })
    }

    pub fn with_cli_detection(cli_detection: CliDetectionResult) -> Self {
        Self { cli_detection }
    }
}

/// POST /api/chat/native
/// Native chat handler that calls Claude CLI directly
pub async fn chat_handler(
    Extension(state): Extension<ChatHandlerState>,
    Json(request): Json<ChatRequest>,
) -> Result<Response, StatusCode> {
    // Use client-provided request_id if available, else generate new one
    let request_id: RequestId = request.request_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    tracing::info!(
        request_id = %request_id,
        session_id = ?request.session_id,
        working_directory = ?request.working_directory,
        "Starting chat request"
    );

    // Build execute options
    let mut options = ExecuteOptions::new(&request.message);

    if let Some(session_id) = &request.session_id {
        options = options.with_session_id(session_id);
    }

    if let Some(cwd) = &request.working_directory {
        options = options.with_working_directory(PathBuf::from(cwd));
    }

    if let Some(tools) = request.allowed_tools {
        options = options.with_allowed_tools(tools);
    }

    if let Some(mode_str) = &request.permission_mode {
        let mode = match mode_str.as_str() {
            "plan" => PermissionMode::Plan,
            "acceptEdits" => PermissionMode::AcceptEdits,
            _ => PermissionMode::Default,
        };
        options = options.with_permission_mode(mode);
    }

    // Add image paths for vision capabilities
    if !request.image_paths.is_empty() {
        let paths: Vec<PathBuf> = request.image_paths.iter()
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .collect();

        if !paths.is_empty() {
            tracing::info!(
                request_id = %request_id,
                image_count = paths.len(),
                "Adding images to request"
            );
            options = options.with_images(paths);
        } else {
            tracing::warn!(
                request_id = %request_id,
                "All provided image paths were invalid or missing"
            );
        }
    }

    // Create executor and spawn process
    let executor = ClaudeExecutor::new(&state.cli_detection);

    let mut process = executor.execute(options).await.map_err(|e| {
        tracing::error!(request_id = %request_id, "Failed to execute Claude CLI: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Register process for abort handling
    let _session_manager = get_session_manager();
    // Note: We can't register the process here because we're moving it into the stream
    // The abort functionality would need a different approach (e.g., using a shared AbortHandle)

    // Create response stream
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, std::io::Error>>(100);

    // Spawn task to stream output
    let req_id_clone = request_id.clone();
    tokio::spawn(async move {
        while let Some(result) = process.recv().await {
            match result {
                Ok(line) => {
                    // Parse and re-serialize as StreamResponse
                    let response = if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&line) {
                        StreamResponse::claude_json(json_value)
                    } else {
                        // If not valid JSON, wrap as error
                        tracing::warn!(request_id = %req_id_clone, "Received non-JSON line: {}", line);
                        continue;
                    };

                    if let Ok(ndjson) = response.to_ndjson() {
                        let line_with_newline = format!("{}\n", ndjson);
                        if tx.send(Ok(line_with_newline)).await.is_err() {
                            tracing::debug!(request_id = %req_id_clone, "Client disconnected");
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(request_id = %req_id_clone, "Error reading from process: {}", e);
                    let error_response = StreamResponse::error(e.to_string());
                    if let Ok(ndjson) = error_response.to_ndjson() {
                        let _ = tx.send(Ok(format!("{}\n", ndjson))).await;
                    }
                    break;
                }
            }
        }

        // Send done message
        let done_response = StreamResponse::done();
        if let Ok(ndjson) = done_response.to_ndjson() {
            let _ = tx.send(Ok(format!("{}\n", ndjson))).await;
        }

        tracing::info!(request_id = %req_id_clone, "Chat request completed");
    });

    // Build streaming response
    let stream = ReceiverStream::new(rx);
    let body = Body::from_stream(stream);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/x-ndjson")
        .header("Transfer-Encoding", "chunked")
        .header("X-Request-Id", request_id)
        .header("Cache-Control", "no-cache")
        .body(body)
        .map_err(|e| {
            tracing::error!("Failed to build response: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(response)
}

/// GET /api/chat/status
/// Check if Claude CLI is available
#[derive(Serialize)]
pub struct ChatStatusResponse {
    pub available: bool,
    pub cli_path: Option<String>,
    pub cli_version: Option<String>,
    pub cli_type: Option<String>,
}

pub async fn chat_status_handler(
    Extension(state): Extension<ChatHandlerState>,
) -> Json<ChatStatusResponse> {
    Json(ChatStatusResponse {
        available: true,
        cli_path: Some(state.cli_detection.executable_path.display().to_string()),
        cli_version: Some(state.cli_detection.version.clone()),
        cli_type: Some(state.cli_detection.cli_type.command_name().to_string()),
    })
}

/// Check CLI availability (for initialization)
pub async fn check_cli_available() -> Result<ChatHandlerState, String> {
    ChatHandlerState::new().await
}
