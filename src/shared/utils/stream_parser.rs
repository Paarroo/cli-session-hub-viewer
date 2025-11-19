use crate::domain::models::{
    AssistantMessage, ContentItem, LogLevel, Message, MessageMetadata, SDKMessage, StreamResponse,
    TodoItem,
};
use chrono::Utc;
use dioxus::prelude::*;
use tracing;

/// Process a single NDJSON line from the stream
pub fn process_stream_line(
    line: &str,
    mut messages: Signal<Vec<Message>>,
    mut current_assistant_message: Signal<Option<Message>>,
    mut current_session_id: Signal<Option<String>>,
    mut is_loading: Signal<bool>,
) {
    // Parse NDJSON line
    let stream_response: StreamResponse = match serde_json::from_str(line) {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to parse stream line: {} - Line: {}", e, line);
            return;
        }
    };

    match stream_response {
        StreamResponse::ClaudeJson { data } => {
            process_claude_sdk_message(
                data,
                messages,
                current_assistant_message,
                current_session_id,
            );
        }

        StreamResponse::Error { error } => {
            messages.write().push(Message::System {
                content: format!("Error: {}", error),
                timestamp: Utc::now(),
                level: Some(LogLevel::Error),
                metadata: None,
            });
            is_loading.set(false);
        }

        StreamResponse::Done => {
            // Finaliser assistant message
            if let Some(msg) = current_assistant_message() {
                messages.write().push(msg);
                current_assistant_message.set(None);
            }
            is_loading.set(false);
        }

        StreamResponse::Aborted => {
            messages.write().push(Message::System {
                content: "Operation was aborted by user".to_string(),
                timestamp: Utc::now(),
                level: Some(LogLevel::Warn),
                metadata: None,
            });
            current_assistant_message.set(None);
            is_loading.set(false);
        }
    }
}

/// Process Claude SDK message and update state accordingly
pub fn process_claude_sdk_message(
    sdk_message: SDKMessage,
    mut messages: Signal<Vec<Message>>,
    mut current_assistant_message: Signal<Option<Message>>,
    mut current_session_id: Signal<Option<String>>,
) {
    match sdk_message.message_type.as_str() {
        "system" => {
            // Extract session_id for continuity
            if let Some(session_id) = sdk_message.session_id {
                current_session_id.set(Some(session_id));
            }

            // Add system message
            let mut content_parts = vec![];
            if let Some(cwd) = sdk_message.cwd {
                content_parts.push(format!("Working directory: {}", cwd));
            }
            if let Some(tools) = sdk_message.tools {
                content_parts.push(format!("Available tools: {}", tools.join(", ")));
            }

            if !content_parts.is_empty() {
                messages.write().push(Message::System {
                    content: content_parts.join("\n"),
                    timestamp: Utc::now(),
                    level: Some(LogLevel::Info),
                    metadata: None,
                });
            }
        }

        "assistant" => {
            if let Some(assistant_msg) = sdk_message.message {
                process_assistant_content(
                    assistant_msg,
                    messages,
                    current_assistant_message,
                );
            }
        }

        "result" => {
            // Finaliser assistant message
            if let Some(msg) = current_assistant_message() {
                messages.write().push(msg);
                current_assistant_message.set(None);
            }

            // Add result message
            let result_text = match sdk_message.subtype.as_deref() {
                Some("success") => "✅ Execution completed successfully",
                Some("error") => "❌ Execution failed",
                _ => "Execution completed",
            };

            messages.write().push(Message::System {
                content: result_text.to_string(),
                timestamp: Utc::now(),
                level: Some(LogLevel::Info),
                metadata: None,
            });
        }

        "user" => {
            // Echo user message (optionnel, généralement déjà ajouté côté client)
            if let Some(text) = sdk_message.text {
                messages.write().push(Message::User {
                    content: text,
                    timestamp: Utc::now(),
                    images: vec![],
                    metadata: None,
                });
            }
        }

        _ => {
            tracing::warn!("Unknown message type: {}", sdk_message.message_type);
        }
    }
}

/// Process assistant message content array
fn process_assistant_content(
    assistant_msg: AssistantMessage,
    mut messages: Signal<Vec<Message>>,
    mut current_assistant_message: Signal<Option<Message>>,
) {
    for item in assistant_msg.content {
        match item.item_type.as_str() {
            "text" => {
                let text_content = item.text.unwrap_or_default();

                // Accumulate text in current_assistant_message
                if let Some(mut msg) = current_assistant_message() {
                    if let Message::Assistant { content, .. } = &mut msg {
                        content.push_str(&text_content);
                    }
                    current_assistant_message.set(Some(msg));
                } else {
                    // Create new assistant message
                    current_assistant_message.set(Some(Message::Assistant {
                        content: text_content,
                        timestamp: Utc::now(),
                        model: None,
                        images: vec![],
                        metadata: None,
                    }));
                }
            }

            "thinking" => {
                // Add thinking message immediately
                messages.write().push(Message::Thinking {
                    content: item.thinking.unwrap_or_default(),
                    timestamp: Utc::now(),
                    metadata: None,
                });
            }

            "tool_use" => {
                let tool_name = item.name.unwrap_or_default();
                let tool_use_id = item.id.clone();

                // Check for ExitPlanMode
                if tool_name == "ExitPlanMode" {
                    if let Some(input) = &item.input {
                        if let Some(plan) = input.get("plan").and_then(|p| p.as_str()) {
                            messages.write().push(Message::Plan {
                                content: plan.to_string(),
                                tool_use_id: tool_use_id.unwrap_or_default(),
                                timestamp: Utc::now(),
                                metadata: None,
                            });
                            return;
                        }
                    }
                }

                // Check for TodoWrite
                if tool_name == "TodoWrite" {
                    if let Some(input) = &item.input {
                        if let Some(todos_array) = input.get("todos").and_then(|t| t.as_array()) {
                            let todo_items: Vec<TodoItem> = todos_array
                                .iter()
                                .filter_map(|todo| {
                                    Some(TodoItem {
                                        content: todo.get("content")?.as_str()?.to_string(),
                                        status: todo.get("status")?.as_str()?.to_string(),
                                        active_form: todo.get("activeForm")?.as_str()?.to_string(),
                                    })
                                })
                                .collect();

                            messages.write().push(Message::Todo {
                                items: todo_items,
                                timestamp: Utc::now(),
                                metadata: None,
                            });
                            return;
                        }
                    }
                }

                // Check for permission error
                if let Some(error) = &item.error {
                    if error.error_type == "PermissionError" {
                        let patterns = error.patterns.clone().unwrap_or_default();
                        let error_msg = format!(
                            "🔐 Permission required for tool: {}\nPatterns: {}",
                            tool_name,
                            patterns.join(", ")
                        );

                        messages.write().push(Message::System {
                            content: error_msg,
                            timestamp: Utc::now(),
                            level: Some(LogLevel::Warn),
                            metadata: None,
                        });

                        // TODO: Trigger permission dialog
                        // context.onPermissionError(tool_name, patterns, tool_use_id)

                        return;
                    }
                }

                // Regular tool use message
                messages.write().push(Message::Tool {
                    name: tool_name,
                    input: item.input.unwrap_or(serde_json::json!({})),
                    output: None,
                    timestamp: Utc::now(),
                    tool_call_id: tool_use_id,
                    metadata: None,
                });
            }

            _ => {
                tracing::warn!("Unknown content type: {}", item.item_type);
            }
        }
    }
}
