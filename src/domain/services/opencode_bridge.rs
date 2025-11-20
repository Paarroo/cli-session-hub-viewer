use crate::domain::models::{AiExecutionRequest, AiExecutionResponse, AiTool};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::Mutex;

/// OpenCode CLI bridge for executing AI commands
pub struct OpenCodeBridge {
    active_sessions: Arc<Mutex<HashMap<String, ()>>>,
}

impl Default for OpenCodeBridge {
    fn default() -> Self {
        Self {
            active_sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl OpenCodeBridge {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if OpenCode CLI is installed
    pub async fn check_opencode_installed(&self) -> bool {
        Command::new("opencode")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Execute OpenCode with the given request
    pub async fn execute(
        &self,
        request: &AiExecutionRequest,
    ) -> Result<AiExecutionResponse, Box<dyn std::error::Error>> {
        match request.ai_tool {
            AiTool::OpenCode => self.execute_opencode(request).await,
            AiTool::Gemini => Err("OpenCodeBridge only handles OpenCode requests".into()),
            AiTool::ClaudeCode => Err("OpenCodeBridge only handles OpenCode requests".into()),
        }
    }

    async fn execute_opencode(
        &self,
        request: &AiExecutionRequest,
    ) -> Result<AiExecutionResponse, Box<dyn std::error::Error>> {
        // Check if OpenCode is installed
        if !self.check_opencode_installed().await {
            return Ok(AiExecutionResponse {
                success: false,
                output: None,
                error: Some("OpenCode CLI is not installed".to_string()),
                session_id: None,
                execution_time_ms: Some(0),
            });
        }

        // Build command arguments
        let mut cmd = Command::new("opencode");
        cmd.current_dir(&request.project_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add prompt if provided
        if let Some(prompt) = &request.prompt {
            cmd.arg("run").arg(prompt);
        } else {
            cmd.arg("shell"); // Interactive mode
        }

        // Add configuration arguments
        if let Some(config) = &request.config {
            if let Some(model) = &config.model {
                cmd.env("OPENCODE_MODEL", model);
            }
            if let Some(temp) = config.temperature {
                cmd.env("OPENCODE_TEMPERATURE", temp.to_string());
            }
            if let Some(tokens) = config.max_tokens {
                cmd.env("OPENCODE_MAX_TOKENS", tokens.to_string());
            }
        }

        // Execute command
        let start_time = std::time::Instant::now();
        let child = cmd.spawn()?;

        // Generate session ID
        let session_id = format!("opencode_{}", uuid::Uuid::new_v4().simple());

        // Wait for completion or timeout
        let timeout_duration = std::time::Duration::from_secs(300); // 5 minutes timeout
        let result = tokio::time::timeout(timeout_duration, async {
            let output: std::process::Output = child.wait_with_output().await?;
            Ok::<_, std::io::Error>(output)
        })
        .await;

        // Store active session (after timeout, child is still available)
        {
            let mut sessions = self.active_sessions.lock().await;
            sessions.insert(session_id.clone(), ());
        }

        // Remove from active sessions
        {
            let mut sessions = self.active_sessions.lock().await;
            sessions.remove(&session_id);
        }

        let execution_time = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(output)) => {
                let success = output.status.success();
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                let combined_output = if stdout.is_empty() {
                    stderr
                } else if stderr.is_empty() {
                    stdout
                } else {
                    format!("{}\n{}", stdout, stderr)
                };

                Ok(AiExecutionResponse {
                    success,
                    output: Some(combined_output),
                    error: if success {
                        None
                    } else {
                        Some("Command failed".to_string())
                    },
                    session_id: Some(session_id),
                    execution_time_ms: Some(execution_time),
                })
            }
            Ok(Err(e)) => Ok(AiExecutionResponse {
                success: false,
                output: None,
                error: Some(format!("Execution error: {}", e)),
                session_id: Some(session_id),
                execution_time_ms: Some(execution_time),
            }),
            Err(_) => {
                // Timeout - remove from active sessions
                {
                    let mut sessions = self.active_sessions.lock().await;
                    sessions.remove(&session_id);
                }

                Ok(AiExecutionResponse {
                    success: false,
                    output: None,
                    error: Some("Command timed out after 5 minutes".to_string()),
                    session_id: Some(session_id),
                    execution_time_ms: Some(execution_time),
                })
            }
        }
    }

    /// Abort a running OpenCode session
    pub async fn abort_session(
        &self,
        session_id: &str,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let mut sessions = self.active_sessions.lock().await;
        if sessions.remove(session_id).is_some() {
            Ok(true) // Session was active, marked as aborted
        } else {
            Ok(false) // Session not found or already finished
        }
    }

    /// Get list of active sessions
    pub async fn get_active_sessions(&self) -> Vec<String> {
        let sessions = self.active_sessions.lock().await;
        sessions.keys().cloned().collect()
    }

    /// Stream OpenCode output in real-time
    pub async fn execute_streaming<F>(
        &self,
        request: &AiExecutionRequest,
        on_output: F,
    ) -> Result<AiExecutionResponse, Box<dyn std::error::Error>>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        // Similar to execute but with streaming output
        if !self.check_opencode_installed().await {
            return Ok(AiExecutionResponse {
                success: false,
                output: None,
                error: Some("OpenCode CLI is not installed".to_string()),
                session_id: None,
                execution_time_ms: Some(0),
            });
        }

        let mut cmd = Command::new("opencode");
        cmd.current_dir(&request.project_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(prompt) = &request.prompt {
            cmd.arg("run").arg(prompt);
        } else {
            cmd.arg("shell");
        }

        let start_time = std::time::Instant::now();
        let mut child = cmd.spawn()?;

        let session_id = format!("opencode_{}", uuid::Uuid::new_v4().simple());

        // Read stdout in real-time
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Store active session
        {
            let mut sessions = self.active_sessions.lock().await;
            sessions.insert(session_id.clone(), ());
        }

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut output_lines = Vec::new();

        // Read both stdout and stderr concurrently
        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            output_lines.push(line.clone());
                            on_output(line);
                        }
                        Ok(None) => break, // EOF
                        Err(e) => {
                            on_output(format!("Error reading stdout: {}", e));
                            break;
                        }
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            output_lines.push(line.clone());
                            on_output(line);
                        }
                        Ok(None) => break,
                        Err(e) => {
                            on_output(format!("Error reading stderr: {}", e));
                            break;
                        }
                    }
                }
            }
        }

        // Wait for process completion
        let status = child.wait().await?;

        // Remove from active sessions
        {
            let mut sessions = self.active_sessions.lock().await;
            sessions.remove(&session_id);
        }

        let execution_time = start_time.elapsed().as_millis() as u64;
        let combined_output = output_lines.join("\n");

        Ok(AiExecutionResponse {
            success: status.success(),
            output: Some(combined_output),
            error: if status.success() {
                None
            } else {
                Some("Command failed".to_string())
            },
            session_id: Some(session_id),
            execution_time_ms: Some(execution_time),
        })
    }
}

/// Get available OpenCode models
pub async fn get_available_models() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output = Command::new("opencode")
        .arg("models")
        .arg("--list")
        .output()
        .await?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let models: Vec<String> = stdout
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
        Ok(models)
    } else {
        Err("Failed to get OpenCode models".into())
    }
}

/// Get OpenCode configuration
pub async fn get_opencode_config() -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let output = Command::new("opencode")
        .arg("config")
        .arg("--show")
        .output()
        .await?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut config = HashMap::new();

        for line in stdout.lines() {
            if let Some((key, value)) = line.split_once('=') {
                config.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        Ok(config)
    } else {
        Err("Failed to get OpenCode configuration".into())
    }
}
