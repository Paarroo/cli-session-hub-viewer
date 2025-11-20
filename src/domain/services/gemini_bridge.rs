use crate::domain::models::AiExecutionRequest;
use crate::cli::claude::detection::{CliType, detect_cli};
use std::process::Stdio;
use tokio::process::Command;
use futures::stream::{self, Stream};
use std::pin::Pin;

/// Bridge for executing Gemini CLI commands
pub struct GeminiBridge {
    executable_path: std::path::PathBuf,
}

impl GeminiBridge {
    /// Create a new Gemini bridge
    pub async fn new() -> Result<Self, String> {
        let detection = detect_cli(CliType::GeminiCli)
            .await
            .map_err(|e| format!("Failed to detect Gemini CLI: {}", e))?;

        Ok(Self {
            executable_path: detection.executable_path,
        })
    }

    /// Execute a Gemini command and return streaming response
    pub async fn execute_gemini(
        &self,
        request: AiExecutionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, String>> + Send>>, String> {
        let mut cmd = Command::new(&self.executable_path);

        // Build Gemini command arguments
        cmd.arg("code")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose");

        // Set working directory to project path
        cmd.current_dir(&request.project_path);

        // Add the user prompt if provided
        if let Some(prompt) = &request.prompt {
            cmd.arg("-p").arg(prompt);
        }

        // Set up streaming
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn Gemini process: {}", e))?;

        // For now, execute synchronously and return result as a single-item stream
        // TODO: Implement proper streaming for Gemini
        let output = child.wait_with_output().await
            .map_err(|e| format!("Failed to wait for Gemini process: {}", e))?;

        let result = if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("Gemini process failed: {}", stderr))
        };

        let stream = stream::once(async move { result });
        Ok(Box::pin(stream))
    }
}

