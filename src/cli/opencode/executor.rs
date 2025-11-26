//! OpenCode CLI executor module
//!
//! Spawns OpenCode CLI processes and streams their output.

use std::path::{Path, PathBuf};
use async_trait::async_trait;

use super::detection::detect_opencode_cli;
use crate::cli::claude::detection::CliDetectionResult;
use crate::cli::traits::{CliExecutor, CliProvider, ExecuteOptions, ExecutorError};

/// OpenCode CLI executor
pub struct OpenCodeExecutor {
    cli_path: PathBuf,
}

impl OpenCodeExecutor {
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

    /// Try to create an executor by auto-detecting OpenCode CLI
    pub async fn auto_detect() -> Result<Self, ExecutorError> {
        let detection = detect_opencode_cli(None)
            .await
            .map_err(|e| ExecutorError::CliNotFound(format!("OpenCode CLI: {}", e)))?;
        Ok(Self::new(&detection))
    }
}

#[async_trait]
impl CliExecutor for OpenCodeExecutor {
    fn provider(&self) -> CliProvider {
        CliProvider::OpenCode
    }

    fn cli_path(&self) -> &Path {
        &self.cli_path
    }

    fn build_args(&self, options: &ExecuteOptions) -> Vec<String> {
        // Build the message with image references if any
        // OpenCode CLI doesn't have --file flag, but can use Read tool to view images
        let message = if options.image_paths.is_empty() {
            options.message.clone()
        } else {
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
            "run".to_string(),
            "-p".to_string(),
            message,
        ];

        // Add session resume if provided
        if let Some(session_id) = &options.session_id {
            args.push("--session".to_string());
            args.push(session_id.clone());
        }

        // Note: Images are now included in the prompt text above
        // OpenCode will use Read tool to view them

        args
    }
    // execute() uses default implementation from CliExecutor trait
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_args_basic() {
        let executor = OpenCodeExecutor::with_path("/usr/bin/opencode");
        let options = ExecuteOptions::new("Hello");

        let args = executor.build_args(&options);

        assert_eq!(args[0], "run");
        assert_eq!(args[1], "-p");
        assert_eq!(args[2], "Hello");
    }

    #[test]
    fn test_build_args_with_session() {
        let executor = OpenCodeExecutor::with_path("/usr/bin/opencode");
        let options = ExecuteOptions::new("Hello")
            .with_session_id("session123");

        let args = executor.build_args(&options);

        assert!(args.contains(&"--session".to_string()));
        assert!(args.contains(&"session123".to_string()));
    }

    #[test]
    fn test_build_args_with_images() {
        let executor = OpenCodeExecutor::with_path("/usr/bin/opencode");
        let options = ExecuteOptions::new("Analyze this")
            .with_image("/path/to/image.png");

        let args = executor.build_args(&options);

        // Image path should be included in the prompt message
        assert!(args[2].contains("[Image: /path/to/image.png]"));
        assert!(args[2].contains("Analyze this"));
    }
}
