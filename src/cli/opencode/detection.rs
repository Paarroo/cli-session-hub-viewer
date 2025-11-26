//! OpenCode CLI detection module
//!
//! Detects and validates OpenCode CLI installation

use std::path::PathBuf;
use crate::cli::claude::detection::{
    CliDetectionResult, CliType, DetectionError, find_cli_in_path, validate_cli,
};

/// Detect OpenCode CLI with optional custom path
pub async fn detect_opencode_cli(custom_path: Option<PathBuf>) -> Result<CliDetectionResult, DetectionError> {
    let path = if let Some(p) = custom_path {
        p
    } else {
        find_cli_in_path(CliType::OpenCode).await?
    };

    let version = validate_cli(&path).await?;

    Ok(CliDetectionResult {
        executable_path: path,
        version,
        cli_type: CliType::OpenCode,
    })
}

/// Check common installation paths for OpenCode CLI
pub async fn find_opencode_in_common_paths() -> Option<PathBuf> {
    let home = dirs::home_dir()?;

    let common_paths = vec![
        // go install
        home.join("go/bin/opencode"),
        // Direct binary in PATH
        PathBuf::from("/usr/local/bin/opencode"),
        // Homebrew (macOS)
        PathBuf::from("/opt/homebrew/bin/opencode"),
        // Linux common
        home.join(".local/bin/opencode"),
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
    async fn test_opencode_detection_missing() {
        // This test verifies the detection returns NotFound when CLI is missing
        // Actual availability depends on system
        let result = detect_opencode_cli(None).await;
        // Just verify it returns a result (success or error)
        assert!(result.is_ok() || matches!(result, Err(DetectionError::NotFound)));
    }
}
