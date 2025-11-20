use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Session status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Completed,
    Failed,
}

/// Session information for database storage
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub ai_tool: crate::domain::models::AiTool,
    pub message_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub summary: String,
    pub status: SessionStatus,
    #[serde(skip)]
    pub last_message_preview: String,
    #[serde(skip)]
    pub last_time: String,
}

/// Conversation summary matching TypeScript webui API
/// ConversationSummary from shared/types.ts
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiSession {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "startTime")]
    pub start_time: String,
    #[serde(rename = "lastTime")]
    pub last_time: String,
    #[serde(rename = "messageCount")]
    pub message_count: usize,
    #[serde(rename = "lastMessagePreview")]
    pub last_message_preview: String,
}
