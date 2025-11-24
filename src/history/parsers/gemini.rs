//! Gemini CLI parser
//!
//! Parses Gemini session-*.json files from ~/.gemini/tmp/{hash}/chats/

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::history::parser::ConversationFile;

/// Parse Gemini session-*.json files in a project's chats directory and return ConversationFile summaries
pub fn parse_gemini_summaries(project_dir: &Path) -> Result<Vec<ConversationFile>, String> {
    // Gemini: chats/session-*.json files
    // Format: { "sessionId": "...", "messages": [...], "startTime": "...", "lastUpdated": "..." }
    let chats_dir = project_dir.join("chats");
    if !chats_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&chats_dir)
        .map_err(|e| format!("Failed to read chats directory: {}", e))?;

    let mut conversation_files = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !name.starts_with("session-") || !name.ends_with(".json") {
            continue;
        }

        let session_id = crate::history::file_utils::extract_session_id(name, "session-", ".json");

        // Use file modification time as fallback instead of Utc::now()
        let file_mtime_fallback = crate::history::file_utils::get_file_mtime_fallback(&path);

        // Parse Gemini session file to get message count and timestamps
        let (message_count, start_time, last_time, preview) = fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
            .map(|json| {
                let count = json.get("messages")
                    .and_then(|m| m.as_array())
                    .map(|arr| arr.len())
                    .unwrap_or(0);
                let start = json.get("startTime")
                    .and_then(|t| t.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| file_mtime_fallback.clone());
                let internal_last = json.get("lastUpdated")
                    .and_then(|t| t.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                // Use MAX of internal timestamp and file mtime for accurate "time ago"
                let last = crate::history::file_utils::get_max_timestamp(internal_last, &file_mtime_fallback);
                // Get last message preview
                let prev = json.get("messages")
                    .and_then(|m| m.as_array())
                    .and_then(|arr| arr.last())
                    .and_then(|msg| msg.get("content"))
                    .and_then(|c| c.as_str())
                    .map(|s| s.chars().take(100).collect::<String>())
                    .unwrap_or_else(|| "Gemini session".to_string());
                (count, start, last, prev)
            })
            .unwrap_or_else(|| {
                (0, file_mtime_fallback.clone(), file_mtime_fallback.clone(), "Gemini session".to_string())
            });

        conversation_files.push(ConversationFile {
            session_id,
            file_path: path.to_string_lossy().to_string(),
            message_ids: HashSet::new(),
            start_time,
            last_time,
            message_count,
            last_message_preview: preview,
        });
    }

    Ok(conversation_files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_timestamp_parsing() {
        let gemini_base = dirs::home_dir()
            .map(|h| h.join(".gemini/tmp"))
            .unwrap();

        if !gemini_base.exists() {
            println!("Gemini directory not found, skipping test");
            return;
        }

        // Find all project directories
        let entries = std::fs::read_dir(&gemini_base).unwrap();
        let mut all_summaries = Vec::new();

        for entry in entries.flatten() {
            let project_dir = entry.path();
            if project_dir.is_dir() {
                if let Ok(summaries) = parse_gemini_summaries(&project_dir) {
                    all_summaries.extend(summaries);
                }
            }
        }

        println!("\n=== Gemini Sessions (raw from parser) ===");
        for s in &all_summaries {
            println!("Session: {} | start: {} | last: {}",
                     s.session_id.chars().take(20).collect::<String>(),
                     s.start_time,
                     s.last_time);
        }

        // Sort by last_time descending (like group_conversations does)
        let mut sorted = all_summaries.clone();
        sorted.sort_by(|a, b| b.last_time.cmp(&a.last_time));

        println!("\n=== After sorting by last_time DESC ===");
        for s in &sorted {
            println!("Session: {} | last: {}",
                     s.session_id.chars().take(20).collect::<String>(),
                     s.last_time);
        }
    }
}
