//! Project discovery functions
//!
//! Discovers projects across all supported AI tools (Claude, OpenCode, Gemini)

use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};

use crate::domain::services::{find_opencode_projects, find_opencode_sessions};
use crate::domain::models::{Project, AiTool};
use super::path_utils::{get_claude_projects_dir, decode_project_path};

/// Discover all projects across all supported AI tools
pub async fn discover_projects() -> Result<Vec<Project>, Box<dyn std::error::Error>> {
    let mut all_projects = Vec::new();

    // Discover Claude Code projects
    match discover_claude_projects().await {
        Ok(projects) => all_projects.extend(projects),
        Err(e) => tracing::warn!("Failed to discover Claude projects: {}", e),
    }

    // Discover OpenCode projects
    match discover_opencode_projects().await {
        Ok(projects) => all_projects.extend(projects),
        Err(e) => tracing::warn!("Failed to discover OpenCode projects: {}", e),
    }

    // Discover Gemini projects
    match discover_gemini_projects().await {
        Ok(projects) => all_projects.extend(projects),
        Err(e) => tracing::warn!("Failed to discover Gemini projects: {}", e),
    }

    Ok(all_projects)
}

/// Discover Claude Code projects
async fn discover_claude_projects() -> Result<Vec<Project>, Box<dyn std::error::Error>> {
    let mut projects = Vec::new();
    let projects_dir = get_claude_projects_dir()?;

    if !projects_dir.exists() {
        return Ok(projects);
    }

    let entries = fs::read_dir(&projects_dir)?;

    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }

        let encoded_name = entry.file_name().to_string_lossy().to_string();
        let decoded_name = decode_project_path(&encoded_name)
            .unwrap_or_else(|_| encoded_name.clone());

        // Count sessions in this project
        let session_count = if let Ok(sessions) = fs::read_dir(entry.path()) {
            sessions.filter_map(|s| s.ok())
                   .filter(|s| s.path().extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
                   .count() as i32
        } else {
            0
        };

        // Get timestamps from the most recent file
        let (last_modified, created_at) = get_project_timestamps(&entry.path())?;

        projects.push(Project {
            id: format!("claude_{}", encoded_name),
            name: decoded_name,
            path: entry.path().to_string_lossy().to_string(),
            ai_tool: AiTool::ClaudeCode,
            session_count,
            last_modified,
            created_at,
            encoded_name,
        });
    }

    Ok(projects)
}

/// Discover OpenCode projects
async fn discover_opencode_projects() -> Result<Vec<Project>, Box<dyn std::error::Error>> {
    let mut projects = Vec::new();

    // Use OpenCode-specific project discovery
    let opencode_projects = find_opencode_projects(&PathBuf::from("."))?;

    for project_path in opencode_projects {
        let project_name = project_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Count sessions using OpenCode session finder
        let session_count = find_opencode_sessions(&project_path)
            .map(|sessions| sessions.len() as i32)
            .unwrap_or(0);

        let (last_modified, created_at) = get_project_timestamps(&project_path)?;

        projects.push(Project {
            id: format!("opencode_{}", project_name),
            name: project_name.clone(),
            path: project_path.to_string_lossy().to_string(),
            ai_tool: AiTool::OpenCode,
            session_count,
            last_modified,
            created_at,
            encoded_name: project_name,
        });
    }

    Ok(projects)
}

/// Discover Gemini projects
/// Gemini stores sessions in ~/.gemini/tmp/{hash}/chats/session-*.json
async fn discover_gemini_projects() -> Result<Vec<Project>, Box<dyn std::error::Error>> {
    let mut projects = Vec::new();

    // Gemini stores projects in ~/.gemini/tmp/{hash}/chats/
    let gemini_tmp = match dirs::home_dir() {
        Some(home) => home.join(".gemini").join("tmp"),
        None => return Ok(projects),
    };

    if !gemini_tmp.exists() {
        return Ok(projects);
    }

    let entries = fs::read_dir(&gemini_tmp)?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let project_hash = entry.file_name().to_string_lossy().to_string();

        // Skip non-hash directories (like 'bin')
        if project_hash.len() < 32 {
            continue;
        }

        // Count sessions in chats/ subdirectory
        let chats_dir = path.join("chats");
        let session_count = if chats_dir.exists() {
            fs::read_dir(&chats_dir)
                .ok()
                .map(|sessions| {
                    sessions
                        .filter_map(|s| s.ok())
                        .filter(|s| {
                            let name = s.file_name().to_string_lossy().to_string();
                            name.starts_with("session-") && name.ends_with(".json")
                        })
                        .count() as i32
                })
                .unwrap_or(0)
        } else {
            0
        };

        // Skip if no sessions
        if session_count == 0 {
            continue;
        }

        let (last_modified, created_at) = get_project_timestamps(&path)?;

        // Use shortened hash for display name
        let display_name = format!("Gemini-{}", &project_hash[..8]);

        projects.push(Project {
            id: format!("gemini_{}", project_hash),
            name: display_name,
            path: path.to_string_lossy().to_string(),
            ai_tool: AiTool::Gemini,
            session_count,
            last_modified,
            created_at,
            encoded_name: project_hash,
        });
    }

    Ok(projects)
}

/// Get timestamps for a project directory
pub fn get_project_timestamps(project_path: &Path) -> Result<(DateTime<Utc>, DateTime<Utc>), Box<dyn std::error::Error>> {
    let metadata = fs::metadata(project_path)?;
    let last_modified = metadata.modified()
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now());
    let created_at = metadata.created()
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| last_modified);

    Ok((last_modified, created_at))
}
