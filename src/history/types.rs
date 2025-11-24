//! Core types for history parsing
//!
//! Contains all data structures used across history parsing modules

use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Snapshot structure for file-history-snapshot entries
/// The timestamp for these entries is inside this nested object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySnapshot {
    pub timestamp: Option<String>,
    #[serde(rename = "messageId")]
    pub message_id: Option<String>,
}

/// Raw JSONL line structure from Claude history files
/// This captures ALL fields from the JSONL format for proper processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawHistoryLine {
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    pub message: Option<InnerMessage>,
    pub timestamp: Option<String>,
    #[serde(rename = "type")]
    pub entry_type: Option<String>,
    /// Unique identifier for this entry
    pub uuid: Option<String>,
    /// Parent UUID for conversation threading
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    /// Whether this is a sidechain (branch) conversation
    #[serde(rename = "isSidechain")]
    pub is_sidechain: Option<bool>,
    /// Type of user (for user messages)
    #[serde(rename = "userType")]
    pub user_type: Option<String>,
    /// Current working directory
    pub cwd: Option<String>,
    /// Claude version
    pub version: Option<String>,
    /// Request ID for tracking
    #[serde(rename = "requestId")]
    pub request_id: Option<String>,
    /// Snapshot data (for file-history-snapshot entries - timestamp is inside)
    pub snapshot: Option<HistorySnapshot>,
}

impl RawHistoryLine {
    /// Get the effective timestamp from either root level or snapshot
    /// For file-history-snapshot entries, the timestamp is in snapshot.timestamp
    pub fn effective_timestamp(&self) -> Option<&String> {
        self.timestamp.as_ref()
            .or_else(|| self.snapshot.as_ref().and_then(|s| s.timestamp.as_ref()))
    }
}

/// Legacy alias for backward compatibility
pub type ClaudeJsonlEntry = RawHistoryLine;

/// Intermediate structure for grouping algorithm
/// Contains parsed file metadata and message IDs for deduplication
#[derive(Debug, Clone)]
pub struct ConversationFile {
    pub session_id: String,
    pub file_path: String,
    pub message_ids: HashSet<String>,
    pub start_time: String,
    pub last_time: String,
    pub message_count: usize,
    pub last_message_preview: String,
}

/// Content can be either a String (user messages) or an Array (assistant messages)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<InnerContentBlock>),
}

impl Default for MessageContent {
    fn default() -> Self {
        MessageContent::Blocks(Vec::new())
    }
}

impl MessageContent {
    /// Returns an iterator over the content blocks
    /// For Text variant, returns empty iterator (handled separately)
    pub fn iter(&self) -> impl Iterator<Item = &InnerContentBlock> {
        match self {
            MessageContent::Blocks(blocks) => blocks.iter(),
            MessageContent::Text(_) => [].iter(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerMessage {
    pub role: Option<String>,
    #[serde(default)]
    pub content: MessageContent,
    /// Message ID - critical for grouping and timestamp restoration
    pub id: Option<String>,
}

/// Content block inside message.content array
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum InnerContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        content: serde_json::Value,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentBlock {
    Text { text: String },
    ToolUse {
        #[serde(rename = "type")]
        tool_type: String,
        name: String,
        input: serde_json::Value
    },
    ToolResult {
        #[serde(rename = "type")]
        result_type: String,
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationHistory {
    pub session_id: String,
    pub project_path: String,
    pub project_name: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub path: String,
    pub encoded_name: String,
    pub session_count: usize,
    pub ai_tool: crate::domain::models::AiTool,
    pub last_updated: DateTime<Utc>,
}
