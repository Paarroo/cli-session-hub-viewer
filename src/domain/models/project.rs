use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// AI tool type for multi AI support
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AiTool {
    ClaudeCode,
    OpenCode,
    Gemini,
}

/// Project information for database storage
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub ai_tool: AiTool,
    pub session_count: i32,
    pub last_modified: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    #[serde(skip)]
    pub encoded_name: String,
}

/// Project information from API response
/// Matches backend ProjectResponse struct
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiProject {
    pub name: String,
    pub path: String,
    pub session_count: i32,
    pub ai_tool: AiTool,
    pub encoded_name: String,
}
