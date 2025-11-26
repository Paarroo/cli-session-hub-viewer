//! Claude CLI executor module
//!
//! Spawns Claude CLI processes and streams their output as NDJSON.

use std::path::{Path, PathBuf};
use async_trait::async_trait;

use super::detection::CliDetectionResult;
use crate::cli::traits::{CliExecutor, CliProcess, CliProvider};

// Re-export types from traits for backward compatibility
pub use crate::cli::traits::{ExecuteOptions, ExecutorError, PermissionMode};

/// Legacy alias for CliProcess (backward compatibility)
pub type ClaudeProcess = CliProcess;

/// Claude CLI executor
pub struct ClaudeExecutor {
    cli_path: PathBuf,
}

impl ClaudeExecutor {
    /// Create a new executor from a detection result
    pub fn new(detection: &CliDetectionResult) -> Self {
        Self {
            cli_path: detection.executable_path.clone(),
        }
    }

    /// Create a new executor with a specific CLI path
    pub fn with_path(cli_path: impl Into<PathBuf>) -> Self {
        Self {
            cli_path: cli_path.into(),
        }
    }
}

#[async_trait]
impl CliExecutor for ClaudeExecutor {
    fn provider(&self) -> CliProvider {
        CliProvider::Claude
    }

    fn cli_path(&self) -> &Path {
        &self.cli_path
    }

    fn build_args(&self, options: &ExecuteOptions) -> Vec<String> {
        // Build the message with image references if any
        // Claude CLI doesn't have --image flag, but Claude can use Read tool to view images
        let message = if options.image_paths.is_empty() {
            options.message.clone()
        } else {
            // Prepend image paths to the prompt so Claude knows to read them
            let image_refs: Vec<String> = options.image_paths
                .iter()
                .map(|p| format!("[Image: {}]", p.display()))
                .collect();

            // Use default prompt if message is empty (image-only send)
            let user_message = if options.message.trim().is_empty() {
                "Describe this image".to_string()
            } else {
                options.message.clone()
            };

            format!(
                "{}\n\nPlease analyze the image(s) above and respond to: {}",
                image_refs.join("\n"),
                user_message
            )
        };

        let mut args = vec![
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
            "-p".to_string(),
            message,
        ];

        // Add session resume if provided
        if let Some(session_id) = &options.session_id {
            args.push("--resume".to_string());
            args.push(session_id.clone());
        }

        // Add allowed tools if specified
        if let Some(tools) = &options.allowed_tools {
            for tool in tools {
                args.push("--allowed-tool".to_string());
                args.push(tool.clone());
            }
        }

        // Add permission mode if not default
        if options.permission_mode != PermissionMode::Default {
            args.push("--permission-mode".to_string());
            args.push(options.permission_mode.as_str().to_string());
        }

        // Note: Images are now included in the prompt text above
        // Claude will use Read tool to view them

        args
    }
    // execute() uses default implementation from CliExecutor trait
}

/// Stream response types (matching claude-code-webui format)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamResponse {
    /// Raw JSON from Claude SDK
    ClaudeJson { data: serde_json::Value },
    /// Error occurred
    Error { error: String },
    /// Stream completed successfully
    Done,
    /// Stream was aborted
    Aborted,
}

impl StreamResponse {
    pub fn claude_json(data: serde_json::Value) -> Self {
        Self::ClaudeJson { data }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self::Error { error: msg.into() }
    }

    pub fn done() -> Self {
        Self::Done
    }

    pub fn aborted() -> Self {
        Self::Aborted
    }

    /// Convert to NDJSON line
    pub fn to_ndjson(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_options_builder() {
        let opts = ExecuteOptions::new("Hello")
            .with_session_id("abc123")
            .with_working_directory("/tmp")
            .with_permission_mode(PermissionMode::Plan);

        assert_eq!(opts.message, "Hello");
        assert_eq!(opts.session_id, Some("abc123".to_string()));
        assert_eq!(opts.working_directory, Some(PathBuf::from("/tmp")));
        assert_eq!(opts.permission_mode, PermissionMode::Plan);
    }

    #[test]
    fn test_stream_response_serialization() {
        let resp = StreamResponse::claude_json(serde_json::json!({"type": "system"}));
        let json = resp.to_ndjson().unwrap();
        assert!(json.contains("claude_json"));
    }
}
