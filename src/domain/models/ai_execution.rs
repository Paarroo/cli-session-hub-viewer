use serde::{Deserialize, Serialize};
use super::{project::AiTool, session::SessionStatus};
use chrono::{DateTime, Utc};

/// AI execution request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiExecutionRequest {
    pub ai_tool: AiTool,
    pub project_path: String,
    pub prompt: Option<String>,
    pub config: Option<AiConfig>,
}

/// AI configuration (generic)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiConfig {
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub custom_params: Option<serde_json::Value>,
}

/// AI execution response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiExecutionResponse {
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub session_id: Option<String>,
    pub execution_time_ms: Option<u64>,
}

/// OpenCode session configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenCodeSession {
    pub id: String,
    pub project_path: String,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub config: OpenCodeConfig,
}

/// OpenCode configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenCodeConfig {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}
