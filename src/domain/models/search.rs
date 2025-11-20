use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use super::project::AiTool;

/// Search and filtering options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub ai_tool: Option<AiTool>,
    pub project_id: Option<String>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Search result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchResult {
    pub session_id: String,
    pub project_id: String,
    pub ai_tool: AiTool,
    pub message_id: String,
    pub content_snippet: String,
    pub score: f32,
    pub timestamp: DateTime<Utc>,
}
