//! Gemini CLI detection module
//!
//! Detects and validates Gemini CLI installation

use std::path::PathBuf;
use crate::cli::claude::detection::{
    CliDetectionResult, CliType, DetectionError, find_cli_in_path, validate_cli,
};

/// Detect Gemini CLI with optional custom path
pub async fn detect_gemini_cli(custom_path: Option<PathBuf>) -> Result<CliDetectionResult, DetectionError> {
    let path = if let Some(p) = custom_path {
        p
    } else {
        find_cli_in_path(CliType::GeminiCli).await?
    };

    let version = validate_cli(&path).await?;

    Ok(CliDetectionResult {
        executable_path: path,
        version,
        cli_type: CliType::GeminiCli,
    })
}

/// Check common installation paths for Gemini CLI
pub async fn find_gemini_in_common_paths() -> Option<PathBuf> {
    let home = dirs::home_dir()?;

    let common_paths = vec![
        // npm global
        home.join(".npm-global/bin/gemini"),
        // pnpm
        home.join(".local/share/pnpm/gemini"),
        // Direct binary in PATH
        PathBuf::from("/usr/local/bin/gemini"),
        // Homebrew (macOS)
        PathBuf::from("/opt/homebrew/bin/gemini"),
        // Linux common
        home.join(".local/bin/gemini"),
    ];

    for path in common_paths {
        if path.exists() {
            return Some(path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gemini_detection_missing() {
        // This test verifies the detection returns NotFound when CLI is missing
        // Actual availability depends on system
        let result = detect_gemini_cli(None).await;
        // Just verify it returns a result (success or error)
        assert!(result.is_ok() || matches!(result, Err(DetectionError::NotFound)));
    }
}
