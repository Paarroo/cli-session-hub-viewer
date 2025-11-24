//! Claude Code parser
//!
//! Parses Claude Code .jsonl history files from ~/.claude/projects/

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::history::parser::{
    ConversationFile, RawHistoryLine, MessageContent, InnerContentBlock,
};

/// Parse Claude .jsonl files in a project directory and return ConversationFile summaries
pub fn parse_claude_summaries(project_dir: &Path) -> Result<Vec<ConversationFile>, String> {
    let entries = fs::read_dir(project_dir)
        .map_err(|e| format!("Failed to read project directory: {}", e))?;

    let mut conversation_files = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }

        match parse_history_file_metadata(&path) {
            Ok(conv_file) => {
                tracing::debug!(
                    "Parsed file: session={}, last_time='{}', msg_count={}",
                    &conv_file.session_id[..8.min(conv_file.session_id.len())],
                    conv_file.last_time,
                    conv_file.message_count
                );
                conversation_files.push(conv_file);
            }
            Err(e) => tracing::warn!("Failed to parse metadata {}: {}", path.display(), e),
        }
    }

    Ok(conversation_files)
}

/// Parse a JSONL file and extract only metadata for listing (lightweight)
/// This is much faster than full parsing for listing many sessions
pub fn parse_history_file_metadata(file_path: &Path) -> Result<ConversationFile, String> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut message_ids: HashSet<String> = HashSet::new();
    let mut start_time: Option<String> = None;
    let mut last_time: Option<String> = None;
    let mut last_message_preview = String::new();
    let mut message_count = 0;
    let mut timestamps_found = 0;
    let mut parse_errors = 0;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let entry: RawHistoryLine = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(e) => {
                parse_errors += 1;
                tracing::debug!("Parse error in {}: {}", file_path.display(), e);
                continue;
            }
        };

        message_count += 1;

        // Debug: Track timestamp extraction (use effective_timestamp to include snapshot.timestamp)
        if entry.effective_timestamp().is_some() {
            timestamps_found += 1;
        }

        // Track message IDs from assistant messages (critical for grouping)
        if let Some(ref inner_msg) = entry.message {
            if inner_msg.role.as_deref() == Some("assistant") {
                if let Some(ref msg_id) = inner_msg.id {
                    message_ids.insert(msg_id.clone());
                }
            }
        }

        // Track timestamps (use effective_timestamp to include snapshot.timestamp for file-history-snapshot entries)
        if let Some(ts) = entry.effective_timestamp() {
            match &start_time {
                None => start_time = Some(ts.clone()),
                Some(existing) if ts < existing => start_time = Some(ts.clone()),
                _ => {}
            }
            match &last_time {
                None => last_time = Some(ts.clone()),
                Some(existing) if ts > existing => last_time = Some(ts.clone()),
                _ => {}
            }
        }

        // Extract last message preview (from assistant messages)
        if let Some(ref inner_msg) = entry.message {
            if inner_msg.role.as_deref() == Some("assistant") {
                match &inner_msg.content {
                    MessageContent::Text(text) => {
                        last_message_preview = text.chars().take(100).collect();
                    }
                    MessageContent::Blocks(blocks) => {
                        for block in blocks {
                            if let InnerContentBlock::Text { text } = block {
                                last_message_preview = text.chars().take(100).collect();
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Extract session ID from file name
    let session_id = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Use file modification time as fallback instead of Utc::now()
    // This ensures files with no timestamps (like summary-only files) still show correct dates
    let file_mtime_fallback = crate::history::file_utils::get_file_mtime_fallback(file_path);

    tracing::debug!(
        "Parsed {}: {} messages, {} timestamps found, {} parse errors, start={:?}, last={:?}",
        session_id,
        message_count,
        timestamps_found,
        parse_errors,
        start_time,
        last_time
    );

    // Use the MAX of internal timestamp and file mtime for last_time
    // This ensures recently modified files show correct "time ago" even if
    // internal timestamps are older (e.g., resumed sessions)
    let effective_last_time = match &last_time {
        Some(internal_ts) => {
            // Compare internal timestamp with file mtime, use the more recent one
            if internal_ts > &file_mtime_fallback {
                internal_ts.clone()
            } else {
                file_mtime_fallback.clone()
            }
        }
        None => file_mtime_fallback.clone(),
    };

    Ok(ConversationFile {
        session_id,
        file_path: file_path.to_string_lossy().to_string(),
        message_ids,
        start_time: start_time.unwrap_or_else(|| file_mtime_fallback.clone()),
        last_time: effective_last_time,
        message_count,
        last_message_preview: if last_message_preview.is_empty() {
            "No preview available".to_string()
        } else {
            last_message_preview
        },
    })
}
