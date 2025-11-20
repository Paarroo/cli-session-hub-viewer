//! OpenCode project/session discovery functions
//!
//! Note: Actual message parsing is done in history/parsers/opencode.rs

use std::fs;
use std::path::Path;

/// Find OpenCode project directories
pub fn find_opencode_projects(
    base_path: &Path,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut projects = Vec::new();

    // OpenCode might store projects in different locations
    // Look for directories containing OpenCode-specific files
    if let Ok(entries) = fs::read_dir(base_path) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                // Check for OpenCode-specific markers
                let opencode_markers = ["opencode.json", ".opencode", "opencode.log"];
                let has_marker = opencode_markers
                    .iter()
                    .any(|marker| entry.path().join(marker).exists());

                if has_marker {
                    projects.push(entry.path());
                }
            }
        }
    }

    Ok(projects)
}

/// Find OpenCode session files in a project (Server-only - uses glob)
#[cfg(not(target_arch = "wasm32"))]
pub fn find_opencode_sessions(
    project_path: &Path,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut sessions = Vec::new();

    // Look for session files with various extensions
    let session_patterns = ["*.jsonl", "*.log", "*.session"];

    for pattern in &session_patterns {
        if let Ok(entries) = glob::glob(&project_path.join(pattern).to_string_lossy()) {
            for entry in entries.flatten() {
                sessions.push(entry);
            }
        }
    }

    Ok(sessions)
}
