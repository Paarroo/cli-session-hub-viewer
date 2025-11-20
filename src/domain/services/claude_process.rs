use std::process::Stdio;
use tokio::process::Command;

/// Spawn a Claude Code process
pub async fn spawn_claude_process(
    project_path: &str,
    prompt: Option<&str>,
) -> Result<tokio::process::Child, std::io::Error> {
    let mut cmd = Command::new("claude");

    cmd.current_dir(project_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(p) = prompt {
        cmd.arg("-p").arg(p);
    }

    cmd.spawn()
}

/// Check if Claude CLI is installed
pub async fn check_claude_installed() -> bool {
    Command::new("claude")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Execute Claude command and return output
pub async fn execute_claude_command(
    project_path: &str,
    prompt: Option<&str>,
) -> Result<String, std::io::Error> {
    let mut cmd = Command::new("claude");

    cmd.current_dir(project_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(p) = prompt {
        cmd.arg("-p").arg(p);
    }

    let child = cmd.spawn()?;
    let output = child.wait_with_output().await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Ok(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
