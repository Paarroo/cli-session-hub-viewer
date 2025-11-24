//! Conversation grouping algorithm
//! Groups conversation files and removes duplicates from continued sessions
//!
//! When a user continues a conversation, Claude creates multiple session files.
//! The newer files contain all messages from previous sessions plus new ones.
//! This module detects these relationships and keeps only the "final" version.

use std::collections::HashSet;
use super::parser::ConversationFile;
use crate::shared::logging;

/// Summary of a conversation for listing purposes (lightweight)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationSummary {
    pub session_id: String,
    pub start_time: String,
    pub last_time: String,
    pub message_count: usize,
    pub last_message_preview: String,
}

/// Check if one set is a subset of another
fn is_subset<T: Eq + std::hash::Hash>(subset: &HashSet<T>, superset: &HashSet<T>) -> bool {
    if subset.len() > superset.len() {
        return false;
    }
    subset.iter().all(|item| superset.contains(item))
}

/// Group conversations and remove duplicates from continued sessions
///
/// Algorithm:
/// 1. Sort conversations by message ID set size (ascending)
/// 2. For each conversation, check if its message IDs are a subset of any larger conversation
/// 3. If yes, it's a duplicate (earlier version of a continued conversation) - skip it
/// 4. If no, it's unique - keep it
///
/// NOTE: Sessions with empty message_ids (OpenCode, Gemini) are never grouped - each is unique
pub fn group_conversations(conversation_files: Vec<ConversationFile>) -> Vec<ConversationSummary> {
    if conversation_files.is_empty() {
        return Vec::new();
    }

    logging::log_grouping_start(conversation_files.len());

    // Sort conversations by message ID set size DESCENDING (largest first)
    // This ensures we process the "final" versions (with most messages) first
    let mut sorted_conversations = conversation_files;
    sorted_conversations.sort_by(|a, b| b.message_ids.len().cmp(&a.message_ids.len()));

    // Remove conversations whose message ID sets are subsets of larger ones
    let mut unique_conversations: Vec<&ConversationFile> = Vec::new();

    for current_conv in &sorted_conversations {
        // Sessions with empty message_ids (OpenCode, Gemini) are always unique
        // Don't apply deduplication logic to them
        if current_conv.message_ids.is_empty() {
            unique_conversations.push(current_conv);
            continue;
        }

        // Check if this conversation's message IDs are a subset of any existing unique conversation
        // Since we process largest first, any current conv that is a subset of an existing one is a duplicate
        let is_subset_of_existing = unique_conversations.iter().any(|existing_conv| {
            // Only compare with conversations that also have message_ids
            !existing_conv.message_ids.is_empty()
                && is_subset(&current_conv.message_ids, &existing_conv.message_ids)
        });

        if !is_subset_of_existing {
            // This is either a unique conversation or the "final" version of a continued conversation
            unique_conversations.push(current_conv);
        }
    }

    // Convert to ConversationSummary and sort by start time (newest first)
    let mut summaries: Vec<ConversationSummary> = unique_conversations
        .iter()
        .map(|conv| create_conversation_summary(conv))
        .collect();

    // Sort by last activity time, newest first (most recently active discussions on top)
    summaries.sort_by(|a, b| b.last_time.cmp(&a.last_time));

    let input_count = sorted_conversations.len();
    let output_count = summaries.len();
    let duplicates_removed = input_count.saturating_sub(output_count);
    logging::log_grouping_result(input_count, output_count, duplicates_removed);

    summaries
}

/// Create a ConversationSummary from a ConversationFile
fn create_conversation_summary(conversation_file: &ConversationFile) -> ConversationSummary {
    ConversationSummary {
        session_id: conversation_file.session_id.clone(),
        start_time: conversation_file.start_time.clone(),
        last_time: conversation_file.last_time.clone(),
        message_count: conversation_file.message_count,
        last_message_preview: conversation_file.last_message_preview.clone(),
    }
}

/// Debug helper to analyze conversation relationships
/// Useful for understanding how conversations are grouped
#[allow(dead_code)]
pub fn analyze_conversation_relationships(
    conversation_files: &[ConversationFile],
) -> ConversationAnalysis {
    let relationships: Vec<ConversationRelationship> = conversation_files
        .iter()
        .map(|conv| {
            let is_subset_of: Vec<String> = conversation_files
                .iter()
                .filter(|other| {
                    conv.session_id != other.session_id
                        && is_subset(&conv.message_ids, &other.message_ids)
                })
                .map(|other| other.session_id.clone())
                .collect();

            ConversationRelationship {
                session_id: conv.session_id.clone(),
                message_id_count: conv.message_ids.len(),
                is_subset_of,
            }
        })
        .collect();

    let duplicate_files: Vec<String> = relationships
        .iter()
        .filter(|rel| !rel.is_subset_of.is_empty())
        .map(|rel| rel.session_id.clone())
        .collect();

    let unique_conversations = conversation_files.len() - duplicate_files.len();

    ConversationAnalysis {
        total_files: conversation_files.len(),
        unique_conversations,
        duplicate_files,
        relationships,
    }
}

#[derive(Debug, Clone)]
pub struct ConversationRelationship {
    pub session_id: String,
    pub message_id_count: usize,
    pub is_subset_of: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConversationAnalysis {
    pub total_files: usize,
    pub unique_conversations: usize,
    pub duplicate_files: Vec<String>,
    pub relationships: Vec<ConversationRelationship>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn make_conv_file(session_id: &str, message_ids: Vec<&str>) -> ConversationFile {
        ConversationFile {
            session_id: session_id.to_string(),
            file_path: format!("/test/{}.jsonl", session_id),
            message_ids: message_ids.into_iter().map(String::from).collect(),
            start_time: "2025-01-01T10:00:00Z".to_string(),
            last_time: "2025-01-01T12:00:00Z".to_string(),
            message_count: 10,
            last_message_preview: "Test preview".to_string(),
        }
    }

    #[test]
    fn test_is_subset() {
        let small: HashSet<i32> = vec![1, 2].into_iter().collect();
        let large: HashSet<i32> = vec![1, 2, 3, 4].into_iter().collect();

        assert!(is_subset(&small, &large));
        assert!(!is_subset(&large, &small));
    }

    #[test]
    fn test_group_conversations_removes_duplicates() {
        let conversations = vec![
            make_conv_file("session-1", vec!["msg-a", "msg-b"]),
            make_conv_file("session-2", vec!["msg-a", "msg-b", "msg-c"]), // Superset of session-1
        ];

        let grouped = group_conversations(conversations);

        // Should only keep session-2 (the larger one)
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[0].session_id, "session-2");
    }

    #[test]
    fn test_group_conversations_keeps_unrelated() {
        let conversations = vec![
            make_conv_file("session-1", vec!["msg-a", "msg-b"]),
            make_conv_file("session-2", vec!["msg-c", "msg-d"]), // Different messages
        ];

        let grouped = group_conversations(conversations);

        // Should keep both (they're unrelated)
        assert_eq!(grouped.len(), 2);
    }

    #[test]
    fn test_analyze_relationships() {
        let conversations = vec![
            make_conv_file("session-1", vec!["msg-a", "msg-b"]),
            make_conv_file("session-2", vec!["msg-a", "msg-b", "msg-c"]),
        ];

        let analysis = analyze_conversation_relationships(&conversations);

        assert_eq!(analysis.total_files, 2);
        assert_eq!(analysis.unique_conversations, 1);
        assert_eq!(analysis.duplicate_files.len(), 1);
        assert!(analysis.duplicate_files.contains(&"session-1".to_string()));
    }
}
