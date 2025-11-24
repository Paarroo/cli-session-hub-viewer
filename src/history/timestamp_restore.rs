//! Timestamp restoration utilities
//! Handles restoring accurate timestamps for continued conversations
//!
//! When conversations are continued, timestamps get overwritten.
//! This module restores original timestamps from first occurrence of each message.id

use std::collections::HashMap;
use super::parser::RawHistoryLine;

/// Restore accurate timestamps for messages in a conversation
/// When conversations are continued, timestamps get overwritten
/// This function restores original timestamps from first occurrence of each message.id
pub fn restore_timestamps(messages: Vec<RawHistoryLine>) -> Vec<RawHistoryLine> {
    // Create a map to track the earliest timestamp for each message ID
    let mut timestamp_map: HashMap<String, String> = HashMap::new();

    // First pass: collect earliest timestamps for each message.id
    for msg in &messages {
        if msg.entry_type.as_deref() == Some("assistant") {
            if let Some(ref inner_msg) = msg.message {
                if let Some(ref msg_id) = inner_msg.id {
                    let current_ts = msg.timestamp.as_deref().unwrap_or("");

                    timestamp_map
                        .entry(msg_id.clone())
                        .and_modify(|existing| {
                            // Keep the earliest timestamp
                            if current_ts < existing.as_str() {
                                *existing = current_ts.to_string();
                            }
                        })
                        .or_insert_with(|| current_ts.to_string());
                }
            }
        }
    }

    // Second pass: restore timestamps and return updated messages
    messages
        .into_iter()
        .map(|mut msg| {
            if msg.entry_type.as_deref() == Some("assistant") {
                if let Some(ref inner_msg) = msg.message {
                    if let Some(ref msg_id) = inner_msg.id {
                        if let Some(restored_ts) = timestamp_map.get(msg_id) {
                            msg.timestamp = Some(restored_ts.clone());
                        }
                    }
                }
            }
            // For user messages and messages without IDs, keep original timestamp
            msg
        })
        .collect()
}

/// Sort messages by timestamp (chronological order)
pub fn sort_by_timestamp(mut messages: Vec<RawHistoryLine>) -> Vec<RawHistoryLine> {
    messages.sort_by(|a, b| {
        let ts_a = a.timestamp.as_deref().unwrap_or("");
        let ts_b = b.timestamp.as_deref().unwrap_or("");
        ts_a.cmp(ts_b)
    });
    messages
}

/// Calculate conversation metadata from messages
pub fn calculate_metadata(messages: &[RawHistoryLine]) -> ConversationMetadata {
    if messages.is_empty() {
        let now = chrono::Utc::now().to_rfc3339();
        return ConversationMetadata {
            start_time: now.clone(),
            end_time: now,
            message_count: 0,
        };
    }

    let mut start_time: Option<&str> = None;
    let mut end_time: Option<&str> = None;

    for msg in messages {
        if let Some(ref ts) = msg.timestamp {
            match start_time {
                None => start_time = Some(ts),
                Some(existing) if ts.as_str() < existing => start_time = Some(ts),
                _ => {}
            }
            match end_time {
                None => end_time = Some(ts),
                Some(existing) if ts.as_str() > existing => end_time = Some(ts),
                _ => {}
            }
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    ConversationMetadata {
        start_time: start_time.unwrap_or(&now).to_string(),
        end_time: end_time.unwrap_or(&now).to_string(),
        message_count: messages.len(),
    }
}

/// Conversation metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationMetadata {
    pub start_time: String,
    pub end_time: String,
    pub message_count: usize,
}

/// Process messages with timestamp restoration and sorting
/// This is the main function to call for preparing messages for API response
pub fn process_conversation_messages(
    messages: Vec<RawHistoryLine>,
) -> (Vec<RawHistoryLine>, ConversationMetadata) {
    // Restore timestamps
    let restored = restore_timestamps(messages);

    // Sort by timestamp
    let sorted = sort_by_timestamp(restored);

    // Calculate metadata
    let metadata = calculate_metadata(&sorted);

    (sorted, metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::parser::{RawHistoryLine, InnerMessage, MessageContent};

    fn make_assistant_msg(id: &str, timestamp: &str) -> RawHistoryLine {
        RawHistoryLine {
            session_id: Some("test".to_string()),
            message: Some(InnerMessage {
                role: Some("assistant".to_string()),
                content: MessageContent::Text("test".to_string()),
                id: Some(id.to_string()),
            }),
            timestamp: Some(timestamp.to_string()),
            entry_type: Some("assistant".to_string()),
            uuid: None,
            parent_uuid: None,
            is_sidechain: None,
            user_type: None,
            cwd: None,
            version: None,
            request_id: None,
            snapshot: None,
        }
    }

    #[test]
    fn test_restore_timestamps_keeps_earliest() {
        let messages = vec![
            make_assistant_msg("msg-1", "2025-01-01T10:00:00Z"),
            make_assistant_msg("msg-1", "2025-01-01T12:00:00Z"), // Same ID, later timestamp
        ];

        let restored = restore_timestamps(messages);

        // Both should have the earliest timestamp
        assert_eq!(restored[0].timestamp, Some("2025-01-01T10:00:00Z".to_string()));
        assert_eq!(restored[1].timestamp, Some("2025-01-01T10:00:00Z".to_string()));
    }

    #[test]
    fn test_sort_by_timestamp() {
        let messages = vec![
            make_assistant_msg("msg-2", "2025-01-01T12:00:00Z"),
            make_assistant_msg("msg-1", "2025-01-01T10:00:00Z"),
        ];

        let sorted = sort_by_timestamp(messages);

        assert_eq!(sorted[0].timestamp, Some("2025-01-01T10:00:00Z".to_string()));
        assert_eq!(sorted[1].timestamp, Some("2025-01-01T12:00:00Z".to_string()));
    }
}
