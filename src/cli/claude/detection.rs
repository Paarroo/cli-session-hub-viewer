//! Claude CLI detection module
//!
//! Detects and validates Claude CLI installation across different package managers
//! (npm, pnpm, yarn, asdf, etc.)

use std::path::PathBuf;
use std::process::Stdio;
use thiserror::Error;
use tokio::process::Command;

/// Errors that can occur during CLI detection
#[derive(Error, Debug)]
pub enum DetectionError {
    #[error("Claude CLI not found in PATH")]
    NotFound,

    #[error("Failed to execute command: {0}")]
    ExecutionFailed(String),

    #[error("Invalid CLI version: {0}")]
    InvalidVersion(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result of CLI detection
#[derive(Debug, Clone)]
pub struct CliDetectionResult {
    /// Path to the CLI executable
    pub executable_path: PathBuf,
    /// CLI version string
    pub version: String,
    /// Type of CLI detected
    pub cli_type: CliType,
}

/// Type of AI CLI tool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliType {
    ClaudeCode,
    OpenCode,
    GeminiCli,
}

impl CliType {
    /// Get the command name for this CLI type
    pub fn command_name(&self) -> &'static str {
        match self {
            CliType::ClaudeCode => "claude",
            CliType::OpenCode => "opencode",
            CliType::GeminiCli => "gemini",
        }
    }

    /// Get all supported CLI types
    pub fn all() -> &'static [CliType] {
        &[CliType::ClaudeCode, CliType::OpenCode, CliType::GeminiCli]
    }
}

/// Find CLI executable in PATH using `which` (Unix) or `where` (Windows)
pub async fn find_cli_in_path(cli_type: CliType) -> Result<PathBuf, DetectionError> {
    let command_name = cli_type.command_name();

    #[cfg(unix)]
    let (finder, args) = ("which", vec![command_name]);

    #[cfg(windows)]
    let (finder, args) = ("where", vec![command_name]);

    let output = Command::new(finder)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();

        if !path_str.is_empty() {
            return Ok(PathBuf::from(path_str));
        }
    }

    Err(DetectionError::NotFound)
}

/// Validate CLI by running `--version`
pub async fn validate_cli(path: &PathBuf) -> Result<String, DetectionError> {
    let output = Command::new(path)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            return Ok(version);
        }
    }

    // Try stderr as some CLIs output version there
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stderr.is_empty() && stderr.contains("version") {
        return Ok(stderr);
    }

    Err(DetectionError::InvalidVersion(
        "Could not determine CLI version".to_string(),
    ))
}

/// Detect and validate a specific CLI type
pub async fn detect_cli(cli_type: CliType) -> Result<CliDetectionResult, DetectionError> {
    let path = find_cli_in_path(cli_type).await?;
    let version = validate_cli(&path).await?;

    Ok(CliDetectionResult {
        executable_path: path,
        version,
        cli_type,
    })
}

/// Detect any available CLI (tries Claude, then OpenCode, then Gemini)
pub async fn detect_any_cli() -> Result<CliDetectionResult, DetectionError> {
    for cli_type in CliType::all() {
        if let Ok(result) = detect_cli(*cli_type).await {
            tracing::info!(
                "Detected {} CLI at {:?} (version: {})",
                cli_type.command_name(),
                result.executable_path,
                result.version
            );
            return Ok(result);
        }
    }

    Err(DetectionError::NotFound)
}

/// Detect Claude CLI specifically with custom path support
pub async fn detect_claude_cli(custom_path: Option<PathBuf>) -> Result<CliDetectionResult, DetectionError> {
    let path = if let Some(p) = custom_path {
        p
    } else {
        find_cli_in_path(CliType::ClaudeCode).await?
    };

    let version = validate_cli(&path).await?;

    Ok(CliDetectionResult {
        executable_path: path,
        version,
        cli_type: CliType::ClaudeCode,
    })
}

/// Check common installation paths for Claude CLI
pub async fn find_claude_in_common_paths() -> Option<PathBuf> {
    let home = dirs::home_dir()?;

    let common_paths = vec![
        // npm global
        home.join(".npm-global/bin/claude"),
        // pnpm
        home.join(".local/share/pnpm/claude"),
        // yarn global
        home.join(".yarn/bin/claude"),
        // asdf
        home.join(".asdf/shims/claude"),
        // Homebrew (macOS)
        PathBuf::from("/opt/homebrew/bin/claude"),
        PathBuf::from("/usr/local/bin/claude"),
        // Windows npm global
        #[cfg(windows)]
        home.join("AppData/Roaming/npm/claude.cmd"),
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
    async fn test_cli_type_command_name() {
        assert_eq!(CliType::ClaudeCode.command_name(), "claude");
        assert_eq!(CliType::OpenCode.command_name(), "opencode");
        assert_eq!(CliType::GeminiCli.command_name(), "gemini");
    }
}
