//! CLI executor traits and common types
//!
//! Provides a common interface for different CLI providers (Claude, OpenCode, Gemini)

use std::path::{Path, PathBuf};
use std::process::Stdio;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

/// Supported CLI providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CliProvider {
    #[default]
    Claude,
    OpenCode,
    Gemini,
}

impl CliProvider {
    /// Get the display name for this provider
    pub fn display_name(&self) -> &'static str {
        match self {
            CliProvider::Claude => "Claude",
            CliProvider::OpenCode => "OpenCode",
            CliProvider::Gemini => "Gemini",
        }
    }

    /// Get the executable name for this provider
    pub fn executable_name(&self) -> &'static str {
        match self {
            CliProvider::Claude => "claude",
            CliProvider::OpenCode => "opencode",
            CliProvider::Gemini => "gemini",
        }
    }

    /// Check if this provider supports images in non-interactive mode
    pub fn supports_images(&self) -> bool {
        match self {
            CliProvider::Claude => true,    // --image flag
            CliProvider::OpenCode => true,  // --file flag
            CliProvider::Gemini => false,   // TUI only
        }
    }
}

impl std::fmt::Display for CliProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for CliProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "claude" => Ok(CliProvider::Claude),
            "opencode" => Ok(CliProvider::OpenCode),
            "gemini" => Ok(CliProvider::Gemini),
            _ => Err(format!("Unknown CLI provider: {}", s)),
        }
    }
}

/// Errors that can occur during CLI execution
#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),

    #[error("Process was aborted")]
    Aborted,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Process exited with error: {0}")]
    ProcessError(String),

    #[error("CLI not found: {0}")]
    CliNotFound(String),

    #[error("Feature not supported: {0}")]
    NotSupported(String),
}

/// Permission mode for CLI execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
    #[default]
    Default,
    Plan,
    AcceptEdits,
}

impl PermissionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            PermissionMode::Default => "default",
            PermissionMode::Plan => "plan",
            PermissionMode::AcceptEdits => "acceptEdits",
        }
    }
}

/// Options for executing a CLI command
#[derive(Debug, Clone, Default)]
pub struct ExecuteOptions {
    /// Message/prompt to send
    pub message: String,
    /// Session ID to resume (for conversation continuity)
    pub session_id: Option<String>,
    /// Working directory for file operations
    pub working_directory: Option<PathBuf>,
    /// Allowed tools (if restricted)
    pub allowed_tools: Option<Vec<String>>,
    /// Permission mode
    pub permission_mode: PermissionMode,
    /// Image paths to include for vision capabilities
    pub image_paths: Vec<PathBuf>,
}

impl ExecuteOptions {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            ..Default::default()
        }
    }

    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_working_directory(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    pub fn with_allowed_tools(mut self, tools: Vec<String>) -> Self {
        self.allowed_tools = Some(tools);
        self
    }

    pub fn with_permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = mode;
        self
    }

    /// Add image paths for vision capabilities
    pub fn with_images(mut self, paths: Vec<PathBuf>) -> Self {
        self.image_paths = paths;
        self
    }

    /// Add a single image path
    pub fn with_image(mut self, path: impl Into<PathBuf>) -> Self {
        self.image_paths.push(path.into());
        self
    }
}

/// A running CLI process
pub struct CliProcess {
    pub(crate) child: tokio::process::Child,
    pub(crate) stdout_rx: mpsc::Receiver<Result<String, ExecutorError>>,
    pub(crate) provider: CliProvider,
}

impl CliProcess {
    /// Kill the process
    pub async fn kill(&mut self) -> Result<(), ExecutorError> {
        self.child.kill().await?;
        Ok(())
    }

    /// Receive the next line from stdout
    pub async fn recv(&mut self) -> Option<Result<String, ExecutorError>> {
        self.stdout_rx.recv().await
    }

    /// Get the process ID
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    /// Get the provider
    pub fn provider(&self) -> CliProvider {
        self.provider
    }
}

/// CLI detection result
#[derive(Debug, Clone)]
pub struct CliDetection {
    pub provider: CliProvider,
    pub executable_path: PathBuf,
    pub version: Option<String>,
    pub is_available: bool,
}

/// Trait for CLI executors
#[async_trait]
pub trait CliExecutor: Send + Sync {
    /// Get the CLI provider type
    fn provider(&self) -> CliProvider;

    /// Get the path to the CLI executable
    fn cli_path(&self) -> &Path;

    /// Check if this executor supports images
    fn supports_images(&self) -> bool {
        self.provider().supports_images()
    }

    /// Build command arguments for execution (CLI-specific)
    fn build_args(&self, options: &ExecuteOptions) -> Vec<String>;

    /// Validate options before execution (can be overridden for CLI-specific validation)
    fn validate_options(&self, options: &ExecuteOptions) -> Result<(), ExecutorError> {
        // Default: no validation
        let _ = options;
        Ok(())
    }

    /// Execute the CLI and return a process handle for streaming
    /// Default implementation uses cli_path() and build_args()
    async fn execute(&self, options: ExecuteOptions) -> Result<CliProcess, ExecutorError> {
        // Validate options first
        self.validate_options(&options)?;
        let args = self.build_args(&options);
        let provider = self.provider();
        let cli_path = self.cli_path();

        tracing::debug!("Executing {} CLI: {:?} {:?}", provider, cli_path, args);

        let mut cmd = Command::new(cli_path);
        cmd.args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null());

        // Set working directory if specified
        if let Some(cwd) = &options.working_directory {
            cmd.current_dir(cwd);
        }

        let mut child = cmd.spawn().map_err(|e| {
            ExecutorError::SpawnFailed(format!("Failed to spawn {} CLI: {}", provider, e))
        })?;

        // Take stdout for streaming
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ExecutorError::SpawnFailed("Failed to capture stdout".to_string()))?;

        // Create channel for streaming lines
        let (tx, rx) = mpsc::channel::<Result<String, ExecutorError>>(100);

        // Spawn task to read stdout
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        if tx.send(Ok(line)).await.is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        let _ = tx.send(Err(ExecutorError::IoError(e))).await;
                        break;
                    }
                }
            }
        });

        Ok(CliProcess {
            child,
            stdout_rx: rx,
            provider,
        })
    }

    /// Execute and collect all output (non-streaming)
    async fn execute_collect(&self, options: ExecuteOptions) -> Result<Vec<String>, ExecutorError> {
        let mut process = self.execute(options).await?;
        let mut lines = Vec::new();

        while let Some(result) = process.recv().await {
            match result {
                Ok(line) => lines.push(line),
                Err(e) => return Err(e),
            }
        }

        Ok(lines)
    }
}

/// Detect which CLI providers are available on the system
pub fn detect_available_providers() -> Vec<CliDetection> {
    let mut detections = Vec::new();

    for provider in [CliProvider::Claude, CliProvider::OpenCode, CliProvider::Gemini] {
        let exe_name = provider.executable_name();

        if let Ok(path) = which::which(exe_name) {
            detections.push(CliDetection {
                provider,
                executable_path: path,
                version: None, // Could be populated by running --version
                is_available: true,
            });
        }
    }

    detections
}

/// Get the first available CLI provider, preferring Claude
pub fn get_default_provider() -> Option<CliProvider> {
    let detections = detect_available_providers();

    // Prefer Claude, then OpenCode, then Gemini
    [CliProvider::Claude, CliProvider::OpenCode, CliProvider::Gemini].into_iter().find(|&preferred| detections.iter().any(|d| d.provider == preferred))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_display_name() {
        assert_eq!(CliProvider::Claude.display_name(), "Claude");
        assert_eq!(CliProvider::OpenCode.display_name(), "OpenCode");
        assert_eq!(CliProvider::Gemini.display_name(), "Gemini");
    }

    #[test]
    fn test_provider_supports_images() {
        assert!(CliProvider::Claude.supports_images());
        assert!(CliProvider::OpenCode.supports_images());
        assert!(!CliProvider::Gemini.supports_images());
    }

    #[test]
    fn test_provider_from_str() {
        assert_eq!("claude".parse::<CliProvider>().unwrap(), CliProvider::Claude);
        assert_eq!("OpenCode".parse::<CliProvider>().unwrap(), CliProvider::OpenCode);
        assert_eq!("GEMINI".parse::<CliProvider>().unwrap(), CliProvider::Gemini);
        assert!("unknown".parse::<CliProvider>().is_err());
    }

    #[test]
    fn test_execute_options_builder() {
        let opts = ExecuteOptions::new("Hello")
            .with_session_id("abc123")
            .with_working_directory("/tmp")
            .with_permission_mode(PermissionMode::Plan)
            .with_image("/path/to/image.png");

        assert_eq!(opts.message, "Hello");
        assert_eq!(opts.session_id, Some("abc123".to_string()));
        assert_eq!(opts.working_directory, Some(PathBuf::from("/tmp")));
        assert_eq!(opts.permission_mode, PermissionMode::Plan);
        assert_eq!(opts.image_paths.len(), 1);
    }
}
