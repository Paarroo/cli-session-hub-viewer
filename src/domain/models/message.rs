use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::image::ImageAttachment;

/// Permission mode for operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PermissionMode {
    Read,
    Write,
    Execute,
}

/// Stream chunk for real-time data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum StreamChunk {
    Text { content: String },
    Done,
    Error { message: String },
    Permission { tool: String, patterns: Vec<String> },
}

/// Log level for system messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Additional metadata for messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageMetadata {
    #[serde(default)]
    pub source: Option<String>, // "claude_code" or "opencode"
    #[serde(default)]
    pub raw_data: Option<serde_json::Value>, // Original format preservation
}

/// TODO item for TodoWrite tool
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoItem {
    pub content: String,
    pub status: String, // "pending", "in_progress", "completed"
    pub active_form: String,
}

/// Message in a conversation (supports both Claude Code and OpenCode formats)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Message {
    User {
        content: String,
        timestamp: DateTime<Utc>,
        #[serde(default)]
        images: Vec<ImageAttachment>,
        #[serde(default)]
        metadata: Option<MessageMetadata>,
    },
    Assistant {
        content: String,
        timestamp: DateTime<Utc>,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        images: Vec<ImageAttachment>,  // Images referenced in response
        #[serde(default)]
        metadata: Option<MessageMetadata>,
    },
    Tool {
        name: String,
        input: serde_json::Value,
        output: Option<String>,
        timestamp: DateTime<Utc>,
        #[serde(default)]
        tool_call_id: Option<String>,
        #[serde(default)]
        metadata: Option<MessageMetadata>,
    },
    System {
        content: String,
        timestamp: DateTime<Utc>,
        #[serde(default)]
        level: Option<LogLevel>,
        #[serde(default)]
        metadata: Option<MessageMetadata>,
    },
    /// Thinking message (Claude's internal reasoning)
    Thinking {
        content: String,
        timestamp: DateTime<Utc>,
        #[serde(default)]
        metadata: Option<MessageMetadata>,
    },
    /// Plan message (ExitPlanMode tool)
    Plan {
        content: String,
        tool_use_id: String,
        timestamp: DateTime<Utc>,
        #[serde(default)]
        metadata: Option<MessageMetadata>,
    },
    /// TODO list (TodoWrite tool)
    Todo {
        items: Vec<TodoItem>,
        timestamp: DateTime<Utc>,
        #[serde(default)]
        metadata: Option<MessageMetadata>,
    },
}

/// Conversation (collection of messages)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    pub session_id: String,
    pub messages: Vec<Message>,
}
