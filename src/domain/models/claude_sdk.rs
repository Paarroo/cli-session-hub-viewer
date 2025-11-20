use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Response wrapper from backend stream (NDJSON format)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamResponse {
    /// Claude SDK message
    ClaudeJson { data: SDKMessage },
    /// Error message
    Error { error: String },
    /// Stream completed successfully
    Done,
    /// Stream aborted by user
    Aborted,
}

/// Claude SDK message (simplified from @anthropic-ai/claude-code SDK)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SDKMessage {
    /// Message type: "system", "assistant", "result", "user"
    #[serde(rename = "type")]
    pub message_type: String,

    /// Session ID (present in "system" messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Current working directory (present in "system" messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,

    /// Tools available (present in "system" messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,

    /// Message content (present in "assistant" messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<AssistantMessage>,

    /// Result subtype (present in "result" messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,

    /// Raw message text (present in "user" messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

/// Assistant message with content array
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssistantMessage {
    /// Array of content items (text, thinking, tool_use)
    pub content: Vec<ContentItem>,
}

/// Content item in assistant message
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContentItem {
    /// Content type: "text", "thinking", "tool_use"
    #[serde(rename = "type")]
    pub item_type: String,

    /// Text content (for type="text")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Thinking content (for type="thinking")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,

    /// Tool name (for type="tool_use")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Tool input (for type="tool_use")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,

    /// Tool use ID (for type="tool_use")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Error information (for permission errors)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ToolError>,
}

/// Tool error (permission errors, etc.)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolError {
    /// Error type: "PermissionError", etc.
    #[serde(rename = "type")]
    pub error_type: String,

    /// Patterns requiring permission
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patterns: Option<Vec<String>>,

    /// Tool use ID associated with error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

/// Chat request to backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// User message content
    pub message: String,

    /// Unique request ID (UUID) for abort functionality
    pub request_id: String,

    /// Session ID for conversation continuity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,

    /// Allowed tools/patterns (for permissions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,

    /// Working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,

    /// Permission mode: "default", "plan", "acceptEdits"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,

    /// Hide user message in chat (for "continue" commands)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hide_user_message: Option<bool>,

    /// Image paths for vision capabilities (server filesystem paths)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub image_paths: Vec<String>,

    /// CLI provider to use (claude, opencode, gemini)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli_provider: Option<String>,
}

impl ChatRequest {
    /// Create new chat request with minimal required fields
    pub fn new(message: String, request_id: String) -> Self {
        Self {
            message,
            request_id,
            session_id: None,
            allowed_tools: None,
            working_directory: None,
            permission_mode: Some("default".to_string()),
            hide_user_message: None,
            image_paths: Vec::new(),
            cli_provider: None,
        }
    }

    /// Set session ID
    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Set working directory
    pub fn with_working_directory(mut self, working_directory: String) -> Self {
        self.working_directory = Some(working_directory);
        self
    }

    /// Set image paths for vision capabilities
    pub fn with_images(mut self, image_paths: Vec<String>) -> Self {
        self.image_paths = image_paths;
        self
    }

    /// Set allowed tools
    pub fn with_allowed_tools(mut self, allowed_tools: Vec<String>) -> Self {
        self.allowed_tools = Some(allowed_tools);
        self
    }

    /// Set permission mode
    pub fn with_permission_mode(mut self, permission_mode: String) -> Self {
        self.permission_mode = Some(permission_mode);
        self
    }

    /// Hide user message
    pub fn hide_message(mut self) -> Self {
        self.hide_user_message = Some(true);
        self
    }

    /// Set CLI provider
    pub fn with_cli_provider(mut self, cli_provider: String) -> Self {
        self.cli_provider = Some(cli_provider);
        self
    }
}
