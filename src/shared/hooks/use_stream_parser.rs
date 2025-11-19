use dioxus::prelude::*;
use crate::domain::models::{LogLevel, Message, StreamChunk};
use chrono::Utc;

/// Hook for parsing SSE streaming messages from Claude backend
///
/// Processes JSON lines and accumulates message content progressively
pub fn use_stream_parser() -> StreamParser {
    StreamParser::new()
}

pub struct StreamParser;

impl Default for StreamParser {
    fn default() -> Self {
        Self
    }
}

impl StreamParser {
    pub fn new() -> Self {
        Self
    }

    /// Process a single line of SSE data
    ///
    /// # Arguments
    /// * `line` - JSON string from SSE stream
    /// * `messages` - Signal containing all messages
    /// * `is_streaming` - Signal to indicate streaming status
    pub fn process_line(
        &self,
        line: &str,
        mut messages: Signal<Vec<Message>>,
        mut is_streaming: Signal<bool>,
    ) {
        // Parse JSON line
        match serde_json::from_str::<StreamResponse>(line) {
            Ok(response) => {
                match response {
                    StreamResponse::ClaudeJson { data } => {
                        self.process_claude_data(data, messages);
                    }
                    StreamResponse::Error { error } => {
                        tracing::error!("Stream error: {}", error);
                        // Add error message
                        messages.write().push(Message::System {
                            content: format!("Error: {}", error),
                            timestamp: Utc::now(),
                            level: Some(LogLevel::Error),
                            metadata: None,
                        });
                        is_streaming.set(false);
                    }
                    StreamResponse::Done => {
                        tracing::info!("Stream completed");
                        is_streaming.set(false);
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Failed to parse stream line: {} - {}", e, line);
            }
        }
    }

    /// Process Claude SDK message data
    fn process_claude_data(&self, chunk: StreamChunk, mut messages: Signal<Vec<Message>>) {
        match chunk {
            StreamChunk::Text { content } => {
                // Accumulate text content in the last assistant message
                let mut msgs = messages.write();

                if let Some(Message::Assistant { content: existing_content, .. }) = msgs.last_mut() {
                    // Append to existing assistant message
                    existing_content.push_str(&content);
                } else {
                    // Create new assistant message
                    msgs.push(Message::Assistant {
                        content,
                        timestamp: Utc::now(),
                        model: None,
                        images: vec![],
                        metadata: None,
                    });
                }
            }
            StreamChunk::Done => {
                // Stream completed successfully
                tracing::info!("Claude chunk processing done");
            }
            StreamChunk::Error { message: _ } => {
                // Add error message
                messages.write().push(Message::System {
                    content: "Stream ended".to_string(),
                    timestamp: Utc::now(),
                    level: Some(LogLevel::Info),
                    metadata: None,
                });
            }
            StreamChunk::Permission { tool, patterns } => {
                // Add permission request message (to be handled by UI later)
                tracing::warn!("Permission request: {} - {:?}", tool, patterns);
                messages.write().push(Message::System {
                    content: "Connection established".to_string(),
                    timestamp: Utc::now(),
                    level: Some(LogLevel::Info),
                    metadata: None,
                });
            }
        }
    }
}

/// Response types from SSE stream
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamResponse {
    ClaudeJson { data: StreamChunk },
    Error { error: String },
    Done,
}
