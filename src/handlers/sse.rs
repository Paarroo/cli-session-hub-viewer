//! SSE (Server-Sent Events) handler for real-time CLI → Web synchronization
//! Watches JSONL files for changes and streams new messages to the frontend

use axum::{
    extract::Path,
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse},
};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher, EventKind};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::path::PathBuf;
use std::time::Duration;
use tokio_stream::wrappers::ReceiverStream;

use crate::history::{load_conversation, ContentBlock};

/// SSE event data for new messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseMessage {
    pub event_type: String,
    pub session_id: String,
    pub message_count: usize,
    pub new_messages: Vec<MessageData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageData {
    pub role: String,
    pub content: String,
    pub timestamp: Option<String>,
}

/// Path parameters for SSE subscription
#[derive(Deserialize)]
pub struct SseSubscribePath {
    pub encoded_name: String,
    pub session_id: String,
}

/// SSE endpoint that watches a specific session for changes
/// GET /api/sse/{encoded_name}/{session_id}
pub async fn sse_session_handler(
    Path(params): Path<SseSubscribePath>,
) -> impl IntoResponse {
    let encoded_name = params.encoded_name;
    let session_id = params.session_id;

    tracing::info!(
        "SSE subscription started for session: {} in project: {}",
        session_id,
        encoded_name
    );

    // Create channel for SSE events
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(100);

    // Spawn file watcher task
    tokio::spawn(async move {
        watch_session_file(tx, encoded_name, session_id).await;
    });

    Sse::new(ReceiverStream::new(rx)).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}

/// Watch session JSONL file for changes and send SSE events
async fn watch_session_file(
    tx: tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
    encoded_name: String,
    session_id: String,
) {
    // Build path to JSONL file
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let jsonl_path = PathBuf::from(&home)
        .join(".claude")
        .join("projects")
        .join(&encoded_name)
        .join(format!("{}.jsonl", session_id));

    tracing::info!("Watching file: {:?}", jsonl_path);

    if !jsonl_path.exists() {
        tracing::error!("Session file not found: {:?}", jsonl_path);
        let _ = tx.send(Ok(Event::default()
            .event("error")
            .data("Session file not found"))).await;
        return;
    }

    // Get initial message count
    let mut last_message_count = match load_conversation(&encoded_name, &session_id) {
        Ok(conv) => conv.messages.len(),
        Err(_) => 0,
    };

    tracing::info!(
        "Initial message count for session {}: {}",
        session_id,
        last_message_count
    );

    // Send initial connection event
    let init_msg = SseMessage {
        event_type: "connected".to_string(),
        session_id: session_id.clone(),
        message_count: last_message_count,
        new_messages: vec![],
    };
    let _ = tx.send(Ok(Event::default()
        .event("connected")
        .data(serde_json::to_string(&init_msg).unwrap_or_default()))).await;

    // Create file watcher channel
    let (watcher_tx, mut watcher_rx) = tokio::sync::mpsc::channel(100);

    // Create file watcher
    let watcher_tx_clone = watcher_tx.clone();
    let mut watcher = match RecommendedWatcher::new(
        move |res: Result<notify::Event, notify::Error>| {
            let _ = watcher_tx_clone.blocking_send(res);
        },
        Config::default().with_poll_interval(Duration::from_millis(500)),
    ) {
        Ok(w) => w,
        Err(e) => {
            tracing::error!("Failed to create file watcher: {}", e);
            return;
        }
    };

    // Watch the session file
    if let Err(e) = watcher.watch(&jsonl_path, RecursiveMode::NonRecursive) {
        tracing::error!("Failed to watch file: {}", e);
        return;
    }

    // Also watch parent directory
    if let Some(parent) = jsonl_path.parent() {
        let _ = watcher.watch(parent, RecursiveMode::NonRecursive);
    }

    // Process file change events
    loop {
        tokio::select! {
            Some(event_result) = watcher_rx.recv() => {
                match event_result {
                    Ok(event) => {
                        if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                            tracing::debug!("File change detected: {:?}", event);

                            // Small delay to let file write complete
                            tokio::time::sleep(Duration::from_millis(100)).await;

                            // Reload conversation and check for new messages
                            if let Ok(conv) = load_conversation(&encoded_name, &session_id) {
                                let new_count = conv.messages.len();

                                if new_count > last_message_count {
                                    let new_messages: Vec<MessageData> = conv
                                        .messages
                                        .iter()
                                        .skip(last_message_count)
                                        .map(|m| {
                                            let content = m
                                                .content
                                                .iter()
                                                .filter_map(|c| match c {
                                                    ContentBlock::Text { text } => Some(text.clone()),
                                                    _ => None,
                                                })
                                                .collect::<Vec<_>>()
                                                .join("\n");

                                            MessageData {
                                                role: m.role.clone(),
                                                content,
                                                timestamp: m.timestamp.clone(),
                                            }
                                        })
                                        .collect();

                                    tracing::info!(
                                        "Sending {} new messages for session {}",
                                        new_messages.len(),
                                        session_id
                                    );

                                    let sse_msg = SseMessage {
                                        event_type: "new_messages".to_string(),
                                        session_id: session_id.clone(),
                                        message_count: new_count,
                                        new_messages,
                                    };

                                    let json = serde_json::to_string(&sse_msg).unwrap_or_default();
                                    if tx.send(Ok(Event::default().event("message").data(json))).await.is_err() {
                                        tracing::debug!("SSE client disconnected");
                                        break;
                                    }

                                    last_message_count = new_count;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Watcher error: {}", e);
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                // Send heartbeat
                let heartbeat = SseMessage {
                    event_type: "heartbeat".to_string(),
                    session_id: session_id.clone(),
                    message_count: last_message_count,
                    new_messages: vec![],
                };
                let json = serde_json::to_string(&heartbeat).unwrap_or_default();
                if tx.send(Ok(Event::default().event("heartbeat").data(json))).await.is_err() {
                    tracing::debug!("SSE client disconnected");
                    break;
                }
            }
        }
    }

    tracing::info!("SSE watcher stopped for session: {}", session_id);
}
