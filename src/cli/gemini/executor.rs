//! Gemini CLI executor module
//!
//! Spawns Gemini CLI processes and streams their output.
//! Note: Gemini CLI does NOT support images in headless mode (TUI only).

use std::path::{Path, PathBuf};
use async_trait::async_trait;

use super::detection::detect_gemini_cli;
use crate::cli::claude::detection::CliDetectionResult;
use crate::cli::traits::{CliExecutor, CliProvider, ExecuteOptions, ExecutorError};

/// Gemini CLI executor
///
/// Important: Gemini CLI does NOT support images in non-interactive mode.
/// Images can only be used via drag & drop in the TUI interface.
pub struct GeminiExecutor {
    cli_path: PathBuf,
}

impl GeminiExecutor {
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

    /// Try to create an executor by auto-detecting Gemini CLI
    pub async fn auto_detect() -> Result<Self, ExecutorError> {
        let detection = detect_gemini_cli(None)
            .await
            .map_err(|e| ExecutorError::CliNotFound(format!("Gemini CLI: {}", e)))?;
        Ok(Self::new(&detection))
    }
}

#[async_trait]
impl CliExecutor for GeminiExecutor {
    fn provider(&self) -> CliProvider {
        CliProvider::Gemini
    }

    fn cli_path(&self) -> &Path {
        &self.cli_path
    }

    fn supports_images(&self) -> bool {
        // Gemini CLI does NOT support images in headless mode (TUI only)
        false
    }

    fn validate_options(&self, options: &ExecuteOptions) -> Result<(), ExecutorError> {
        // Reject if images are provided - Gemini CLI doesn't support them in headless mode
        if !options.image_paths.is_empty() {
            return Err(ExecutorError::NotSupported(
                "Gemini CLI ne supporte pas les images en mode headless (TUI uniquement). Utilisez Claude ou OpenCode pour l'analyse d'images.".to_string()
            ));
        }
        Ok(())
    }

    fn build_args(&self, options: &ExecuteOptions) -> Vec<String> {
        // Gemini CLI has limited options in non-interactive mode
        // Session resume and other features may not be available
        vec![
            "-p".to_string(),
            options.message.clone(),
        ]
    }
    // execute() uses default implementation from CliExecutor trait
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_args_basic() {
        let executor = GeminiExecutor::with_path("/usr/bin/gemini");
        let options = ExecuteOptions::new("Hello");

        let args = executor.build_args(&options);

        assert_eq!(args[0], "-p");
        assert_eq!(args[1], "Hello");
    }

    #[test]
    fn test_supports_images() {
        let executor = GeminiExecutor::with_path("/usr/bin/gemini");
        assert!(!executor.supports_images());
    }
}
