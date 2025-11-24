//! OpenCode parser
//!
//! Parses OpenCode ses_*.json session files from ~/.local/share/opencode/storage/session/
//! Messages are stored separately in ~/.local/share/opencode/storage/message/ses_*/msg_*.json
//! Message parts are in ~/.local/share/opencode/storage/part/msg_*/prt_*.json

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::history::parser::{ConversationFile, Message, ContentBlock};
use crate::shared::logging;

/// Parse OpenCode ses_*.json files in a project directory and return ConversationFile summaries
pub fn parse_opencode_summaries(project_dir: &Path) -> Result<Vec<ConversationFile>, String> {
    let entries = fs::read_dir(project_dir)
        .map_err(|e| format!("Failed to read project directory: {}", e))?;

    // Get the base storage directory for message counting
    let opencode_base = dirs::home_dir()
        .map(|h| h.join(".local/share/opencode/storage/message"));

    let mut conversation_files = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.starts_with("ses_") || !name.ends_with(".json") {
            continue;
        }

        // OpenCode uses JSON format - create summary from file metadata
        let session_id = crate::history::file_utils::extract_session_id(name, "ses_", ".json");
        let full_session_id = format!("ses_{}", session_id);

        // Count messages from storage/message/ses_*/ directory
        let message_count = opencode_base.as_ref()
            .and_then(|base| {
                let msg_dir = base.join(&full_session_id);
                if msg_dir.exists() {
                    fs::read_dir(&msg_dir).ok().map(|entries| {
                        entries.filter_map(|e| e.ok())
                            .filter(|e| {
                                let n = e.file_name().to_string_lossy().to_string();
                                n.starts_with("msg_") && n.ends_with(".json")
                            })
                            .count()
                    })
                } else {
                    None
                }
            })
            .unwrap_or(0);

        // Read timestamps from JSON file (time.created, time.updated are Unix milliseconds)
        // Use file modification time as fallback
        let file_mtime_fallback = crate::history::file_utils::get_file_mtime_fallback(&path);

        let (start_time, last_time, title) = fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
            .map(|json| {
                let created = json.get("time")
                    .and_then(|t| t.get("created"))
                    .and_then(|c| c.as_i64())
                    .and_then(chrono::DateTime::from_timestamp_millis)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| file_mtime_fallback.clone());

                let internal_updated = json.get("time")
                    .and_then(|t| t.get("updated"))
                    .and_then(|u| u.as_i64())
                    .and_then(chrono::DateTime::from_timestamp_millis)
                    .map(|dt| dt.to_rfc3339());

                // Use MAX of internal timestamp and file mtime for accurate "time ago"
                let updated = crate::history::file_utils::get_max_timestamp(internal_updated, &file_mtime_fallback);

                let title = json.get("title")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("OpenCode session ({} messages)", message_count));

                (created, updated, title)
            })
            .unwrap_or_else(|| (file_mtime_fallback.clone(), file_mtime_fallback, format!("OpenCode session ({} messages)", message_count)));

        conversation_files.push(ConversationFile {
            session_id,
            file_path: path.to_string_lossy().to_string(),
            message_ids: HashSet::new(),
            start_time,
            last_time,
            message_count,
            last_message_preview: title,
        });
    }

    Ok(conversation_files)
}

/// Load OpenCode messages from the separate message and parts directories
/// Structure:
/// - Messages metadata: ~/.local/share/opencode/storage/message/ses_{id}/msg_*.json
/// - Message content: ~/.local/share/opencode/storage/part/msg_{message_id}/prt_*.json
pub fn load_opencode_messages(session_id: &str) -> Result<Vec<Message>, String> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| "Could not find home directory".to_string())?;

    let storage_base = home_dir.join(".local/share/opencode/storage");

    // OpenCode message directory: ~/.local/share/opencode/storage/message/ses_{id}/
    // Note: session_id comes WITHOUT the "ses_" prefix, so we add it
    let message_dir = storage_base
        .join("message")
        .join(format!("ses_{}", session_id));

    if !message_dir.exists() {
        logging::log_opencode_message_dir(session_id, false, 0);
        return Ok(Vec::new());
    }

    let parts_base = storage_base.join("part");

    let mut messages = Vec::new();
    let mut message_files: Vec<_> = fs::read_dir(&message_dir)
        .map_err(|e| format!("Failed to read message directory: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("msg_") && name.ends_with(".json")
        })
        .collect();

    // Sort by filename to maintain order (msg_*.json files)
    message_files.sort_by_key(|e| e.file_name());

    for entry in message_files {
        let msg_path = entry.path();
        let content = match fs::read_to_string(&msg_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read message file {}: {}", msg_path.display(), e);
                continue;
            }
        };

        let json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(j) => j,
            Err(e) => {
                tracing::warn!("Failed to parse message JSON {}: {}", msg_path.display(), e);
                continue;
            }
        };

        // Extract message ID for looking up parts
        let message_id = json.get("id")
            .and_then(|id| id.as_str())
            .unwrap_or("")
            .to_string();

        // Extract role (user or assistant)
        let role = json.get("role")
            .and_then(|r| r.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Extract timestamp from time.created (milliseconds)
        let timestamp = json.get("time")
            .and_then(|t| t.get("created"))
            .and_then(|c| c.as_i64())
            .map(|ms| {
                chrono::DateTime::from_timestamp_millis(ms)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            });

        // Load actual content based on role
        // - User messages: content is in summary.body (or fallback to summary.title)
        // - Assistant messages: content is in parts directory
        let content_text = if role == "user" {
            // User messages have content directly in the message JSON
            json.get("summary")
                .and_then(|s| {
                    // Try body first, then title
                    s.get("body").and_then(|b| b.as_str())
                        .or_else(|| s.get("title").and_then(|t| t.as_str()))
                })
                .unwrap_or("")
                .to_string()
        } else if !message_id.is_empty() {
            // Assistant messages: load from parts directory
            load_opencode_message_parts(&parts_base, &message_id)
        } else {
            // Fallback to summary.title if no message_id
            json.get("summary")
                .and_then(|s| s.get("title"))
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string()
        };

        // Skip empty messages
        if content_text.is_empty() {
            continue;
        }

        messages.push(Message {
            role,
            content: vec![ContentBlock::Text { text: content_text }],
            timestamp,
        });
    }

    logging::log_opencode_message_dir(session_id, true, messages.len());
    logging::log_session_load_success("opencode", session_id, messages.len());
    Ok(messages)
}

/// Load message content from parts directory.
///
/// Parts are stored in: `~/.local/share/opencode/storage/part/msg_{message_id}/prt_*.json`
///
/// Part types with text content:
/// - `text`: Direct text output from the model
/// - `reasoning`: Model's reasoning/thinking text
///
/// Other types (step-start, step-finish, tool) don't have readable text content.
fn load_opencode_message_parts(parts_base: &Path, message_id: &str) -> String {
    let parts_dir = parts_base.join(message_id);

    if !parts_dir.exists() {
        return String::new();
    }

    let mut parts: Vec<(String, String)> = Vec::new(); // (filename, text)

    if let Ok(entries) = fs::read_dir(&parts_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let filename = entry.file_name().to_string_lossy().to_string();

            if !filename.starts_with("prt_") || !filename.ends_with(".json") {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    let part_type = json.get("type").and_then(|t| t.as_str()).unwrap_or("");

                    // Extract text from "text" and "reasoning" type parts
                    // Both have a "text" field with the actual content
                    if part_type == "text" || part_type == "reasoning" {
                        if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                            if !text.is_empty() {
                                parts.push((filename, text.to_string()));
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort by filename to maintain order
    parts.sort_by(|a, b| a.0.cmp(&b.0));

    // Concatenate all text parts
    parts.into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opencode_timestamp_parsing() {
        let opencode_dir = dirs::home_dir()
            .map(|h| h.join(".local/share/opencode/storage/session/global"))
            .unwrap();

        if !opencode_dir.exists() {
            println!("OpenCode directory not found, skipping test");
            return;
        }

        let summaries = parse_opencode_summaries(&opencode_dir).unwrap();

        println!("\n=== OpenCode Sessions (raw from parser) ===");
        for s in &summaries {
            println!("Session: {} | start: {} | last: {}",
                     s.session_id.chars().take(20).collect::<String>(),
                     s.start_time,
                     s.last_time);
        }

        // Sort by last_time descending (like group_conversations does)
        let mut sorted = summaries.clone();
        sorted.sort_by(|a, b| b.last_time.cmp(&a.last_time));

        println!("\n=== After sorting by last_time DESC ===");
        for s in &sorted {
            println!("Session: {} | last: {}",
                     s.session_id.chars().take(20).collect::<String>(),
                     s.last_time);
        }
    }
}
