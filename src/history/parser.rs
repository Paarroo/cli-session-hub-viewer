//! History parser functions
//!
//! Types are defined in types.rs, this module contains parsing logic

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};

use crate::shared::logging;

use super::path_utils::{get_claude_projects_dir, get_projects_dir, decode_project_path, encode_project_path, smart_decode_project_path, decode_gemini_hash};
use super::grouping::{group_conversations, ConversationSummary};

// Re-export types for backward compatibility
pub use super::types::{
    HistorySnapshot,
    RawHistoryLine,
    ClaudeJsonlEntry,
    ConversationFile,
    MessageContent,
    InnerMessage,
    InnerContentBlock,
    Message,
    ContentBlock,
    ConversationHistory,
    ProjectInfo,
};

/// Parse a single .jsonl file and return conversation history
pub fn parse_jsonl_file(file_path: &Path) -> Result<ConversationHistory, String> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut messages = Vec::new();
    let mut created_at = None;
    let mut updated_at = None;
    let mut real_session_id: Option<String> = None;

    for (line_num, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        // Parse as Claude's JSONL wrapper format
        let entry: ClaudeJsonlEntry = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(e) => {
                tracing::debug!("Skipping line {}: {}", line_num + 1, e);
                continue;
            }
        };

        // Extract real session_id from JSON content
        if real_session_id.is_none() {
            real_session_id = entry.session_id.clone();
        }

        // Track timestamps
        if let Some(ts) = &entry.timestamp {
            if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
                let utc_dt = dt.with_timezone(&Utc);
                if created_at.is_none() {
                    created_at = Some(utc_dt);
                }
                updated_at = Some(utc_dt);
            }
        }

        // Handle user and assistant messages
        // Note: entry.type can be "user" but assistant messages don't have entry.type
        // So we check message.role instead
        let entry_type = entry.entry_type.as_deref();
        tracing::debug!("Processing entry type={:?}, has_message={}", entry_type, entry.message.is_some());

        if let Some(inner) = entry.message {
            let role = inner.role.as_deref();
            tracing::debug!("Entry has message with role={:?}", role);

            // Accept user/assistant messages (check role, not entry type)
            if role == Some("user") || role == Some("assistant") {
                // Extract text content - handle both String (user) and Array (assistant) formats
                let text_content: Vec<ContentBlock> = match &inner.content {
                    // User messages: content is a simple string
                    MessageContent::Text(text) => {
                        vec![ContentBlock::Text { text: text.clone() }]
                    },
                    // Assistant messages: content is an array of blocks
                    MessageContent::Blocks(blocks) => {
                        blocks.iter()
                            .filter_map(|block| {
                                match block {
                                    InnerContentBlock::Text { text } => Some(ContentBlock::Text { text: text.clone() }),
                                    InnerContentBlock::ToolUse { name, input } => Some(ContentBlock::ToolUse {
                                        tool_type: "tool_use".to_string(),
                                        name: name.clone(),
                                        input: input.clone(),
                                    }),
                                    InnerContentBlock::ToolResult { content } => {
                                        let content_str = match content {
                                            serde_json::Value::String(s) => s.clone(),
                                            _ => content.to_string(),
                                        };
                                        Some(ContentBlock::ToolResult {
                                            result_type: "tool_result".to_string(),
                                            content: content_str,
                                        })
                                    },
                                    _ => None, // Skip thinking and other blocks
                                }
                            })
                            .collect()
                    }
                };

                // Include all messages with any content
                if !text_content.is_empty() {
                    let role_str = role.unwrap_or("unknown").to_string();
                    tracing::debug!(
                        "Adding message: role={}, content_blocks={}",
                        role_str,
                        text_content.len()
                    );
                    messages.push(Message {
                        role: role_str,
                        content: text_content,
                        timestamp: entry.timestamp.clone(),
                    });
                } else {
                    tracing::debug!("Skipping empty message for entry_type={:?}", entry_type);
                }
            }
        }
    }

    // Use real session_id from JSON, fallback to filename
    let session_id = real_session_id.unwrap_or_else(|| {
        file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string()
    });

    // Extract project path from parent directory name
    let encoded_name = file_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let project_path = decode_project_path(encoded_name)
        .unwrap_or_else(|_| "unknown".to_string());

    let project_name = Path::new(&project_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(ConversationHistory {
        session_id,
        project_path: project_path.clone(),
        project_name,
        messages: messages.clone(),
        created_at: created_at.unwrap_or_else(Utc::now),
        updated_at: updated_at.unwrap_or_else(Utc::now),
        message_count: messages.len(),
    })
}

/// Read configuration for a specific AI tool
fn read_tool_config(ai_tool: &crate::domain::models::AiTool) -> Result<Option<serde_json::Value>, String> {
    let home = std::env::var("HOME")
        .map_err(|_| "HOME environment variable not set".to_string())?;

    let config_file = match ai_tool {
        crate::domain::models::AiTool::ClaudeCode => ".claude.json",
        crate::domain::models::AiTool::OpenCode => ".opencode.json",
        crate::domain::models::AiTool::Gemini => ".gemini.json",
    };

    let config_path = PathBuf::from(home).join(config_file);

    if !config_path.exists() {
        tracing::info!("Config file not found for {:?}: {}", ai_tool, config_path.display());
        return Ok(None);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config for {:?}: {}", ai_tool, e))?;

    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse config for {:?}: {}", ai_tool, e))?;

    tracing::info!("Successfully read config for {:?} from: {}", ai_tool, config_path.display());
    Ok(Some(config))
}

/// Read Claude configuration from ~/.claude.json (legacy function)
#[allow(dead_code)]
fn read_claude_config() -> Result<Option<serde_json::Value>, String> {
    read_tool_config(&crate::domain::models::AiTool::ClaudeCode)
}

/// Encode a project path to match Claude's directory naming convention
/// This matches the logic from the webui's getEncodedProjectName function
/// Uses the centralized encode_project_path function from path_utils
fn encode_project_path_claude(path: &str) -> String {
    // Delegate to centralized function to avoid duplication
    encode_project_path(path)
}

/// List all projects from all AI tool directories
pub fn list_projects() -> Result<Vec<ProjectInfo>, String> {
    tracing::info!("list_projects() called");
    let mut all_projects = Vec::new();

    // Scan projects for each AI tool
    let ai_tools = vec![
        crate::domain::models::AiTool::ClaudeCode,
        crate::domain::models::AiTool::OpenCode,
        crate::domain::models::AiTool::Gemini,
    ];

    for ai_tool in ai_tools {
        tracing::info!("Scanning projects for tool: {:?}", ai_tool);
        match scan_projects_for_tool(&ai_tool) {
            Ok(mut projects) => {
                tracing::info!("Found {} projects for {:?}: {:?}", projects.len(), ai_tool, projects.iter().map(|p| &p.name).collect::<Vec<_>>());
                all_projects.append(&mut projects);
            }
            Err(e) => {
                tracing::warn!("Failed to scan projects for {:?}: {}", ai_tool, e);
            }
        }
    }

    tracing::info!("Total projects found: {}", all_projects.len());
    Ok(all_projects)
}

/// Extract the working directory from the most recent OpenCode session file
/// OpenCode stores `directory` field in each ses_*.json file
fn extract_opencode_working_directory(session_dir: &Path) -> Option<String> {
    // Read all session files and find the most recent one
    let entries = fs::read_dir(session_dir).ok()?;

    let mut most_recent: Option<(std::time::SystemTime, String)> = None;

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if !name.starts_with("ses_") || !name.ends_with(".json") {
            continue;
        }

        // Get file modification time
        let metadata = fs::metadata(&path).ok()?;
        let modified = metadata.modified().ok()?;

        // Read and parse the JSON to get directory
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(dir) = json.get("directory").and_then(|v| v.as_str()) {
                    match &most_recent {
                        None => most_recent = Some((modified, dir.to_string())),
                        Some((prev_time, _)) if modified > *prev_time => {
                            most_recent = Some((modified, dir.to_string()));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    most_recent.map(|(_, dir)| dir)
}

/// Count sessions in a project directory based on AI tool format
/// - Claude: *.jsonl files directly in project dir
/// - OpenCode: ses_*.json files directly in project dir
/// - Gemini: session-*.json files in chats/ subdirectory
fn count_sessions_for_tool(project_dir: &Path, ai_tool: &crate::domain::models::AiTool) -> usize {
    match ai_tool {
        crate::domain::models::AiTool::ClaudeCode => {
            // Claude: count *.jsonl files
            fs::read_dir(project_dir)
                .ok()
                .map(|entries| {
                    entries
                        .flatten()
                        .filter(|e| {
                            e.path()
                                .extension()
                                .and_then(|ext| ext.to_str())
                                == Some("jsonl")
                        })
                        .count()
                })
                .unwrap_or(0)
        }
        crate::domain::models::AiTool::OpenCode => {
            // OpenCode: count ses_*.json files
            fs::read_dir(project_dir)
                .ok()
                .map(|entries| {
                    entries
                        .flatten()
                        .filter(|e| {
                            let name = e.file_name().to_string_lossy().to_string();
                            name.starts_with("ses_") && name.ends_with(".json")
                        })
                        .count()
                })
                .unwrap_or(0)
        }
        crate::domain::models::AiTool::Gemini => {
            // Gemini: count session-*.json files in chats/ subdirectory
            let chats_dir = project_dir.join("chats");
            if chats_dir.exists() {
                fs::read_dir(&chats_dir)
                    .ok()
                    .map(|entries| {
                        entries
                            .flatten()
                            .filter(|e| {
                                let name = e.file_name().to_string_lossy().to_string();
                                name.starts_with("session-") && name.ends_with(".json")
                            })
                            .count()
                    })
                    .unwrap_or(0)
            } else {
                0
            }
        }
    }
}

/// Get the last updated timestamp for a project directory by checking session file modification times
fn get_project_last_updated(project_dir: &Path, ai_tool: &crate::domain::models::AiTool) -> DateTime<Utc> {
    let mut latest_time: Option<std::time::SystemTime> = None;

    match ai_tool {
        crate::domain::models::AiTool::ClaudeCode => {
            // Check .jsonl files in the project directory
            if let Ok(entries) = fs::read_dir(project_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "jsonl") {
                        if let Ok(metadata) = fs::metadata(&path) {
                            if let Ok(modified) = metadata.modified() {
                                latest_time = Some(match latest_time {
                                    None => modified,
                                    Some(prev) if modified > prev => modified,
                                    Some(prev) => prev,
                                });
                            }
                        }
                    }
                }
            }
        }
        crate::domain::models::AiTool::OpenCode => {
            // Check ses_*.json files
            if let Ok(entries) = fs::read_dir(project_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.starts_with("ses_") && name.ends_with(".json") {
                        if let Ok(metadata) = fs::metadata(&path) {
                            if let Ok(modified) = metadata.modified() {
                                latest_time = Some(match latest_time {
                                    None => modified,
                                    Some(prev) if modified > prev => modified,
                                    Some(prev) => prev,
                                });
                            }
                        }
                    }
                }
            }
        }
        crate::domain::models::AiTool::Gemini => {
            // Read lastUpdated from JSON files (more accurate than file mtime)
            let chats_dir = project_dir.join("chats");
            if let Ok(entries) = fs::read_dir(&chats_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name.starts_with("session-") && name.ends_with(".json") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(last_updated) = json.get("lastUpdated").and_then(|v| v.as_str()) {
                                    if let Ok(dt) = DateTime::parse_from_rfc3339(last_updated) {
                                        let sys_time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(dt.timestamp() as u64);
                                        latest_time = Some(match latest_time {
                                            None => sys_time,
                                            Some(prev) if sys_time > prev => sys_time,
                                            Some(prev) => prev,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Convert SystemTime to DateTime<Utc>
    

    match latest_time {
        Some(time) => {
            let duration = time.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
                .unwrap_or_else(Utc::now)
        }
        None => Utc::now(),
    }
}

/// Scan projects for a specific AI tool
fn scan_projects_for_tool(ai_tool: &crate::domain::models::AiTool) -> Result<Vec<ProjectInfo>, String> {
    let tool_name = format!("{:?}", ai_tool);
    logging::log_project_discovery_start(&tool_name);
    let projects_dir = get_projects_dir(ai_tool)?;
    tracing::info!("Projects directory for {:?}: {}", ai_tool, projects_dir.display());

    if !projects_dir.exists() {
        tracing::info!("Projects directory does not exist for {:?}: {}", ai_tool, projects_dir.display());
        return Ok(Vec::new());
    }

    // First try to read from config (like the webui does)
    let mut projects: HashMap<String, ProjectInfo> = HashMap::new();

    // Read config for this specific tool
    if let Ok(Some(config)) = read_tool_config(ai_tool) {
        tracing::info!("Successfully read config for {:?}, looking for projects section", ai_tool);
        if let Some(projects_config) = config.get("projects").and_then(|p| p.as_object()) {
            tracing::info!("Found {} projects in {:?} config: {:?}", projects_config.len(), ai_tool, projects_config.keys().collect::<Vec<_>>());

            for (path, _value) in projects_config {
                let encoded_name = encode_project_path_claude(path);

                // Check if the encoded directory actually exists
                let project_dir = projects_dir.join(&encoded_name);
                if project_dir.exists() && project_dir.is_dir() {
                    tracing::debug!("Config project exists for {:?}: {} -> {}", ai_tool, path, encoded_name);

                    // Count sessions using tool-specific logic
                    let session_count = count_sessions_for_tool(&project_dir, ai_tool);
                    let last_updated = get_project_last_updated(&project_dir, ai_tool);

                    let project_name = PathBuf::from(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    projects.insert(
                        encoded_name.clone(),
                        ProjectInfo {
                            name: project_name,
                            path: path.clone(),
                            encoded_name,
                            session_count,
                            ai_tool: ai_tool.clone(),
                            last_updated,
                        },
                    );
                } else {
                    tracing::debug!("Config project directory not found for {:?}: {} ({})", ai_tool, path, encoded_name);
                }
            }
        }
    }

    // Special case for OpenCode: the "global" directory contains sessions directly
    // OpenCode stores all sessions in storage/session/global/ with ses_*.json files
    if matches!(ai_tool, crate::domain::models::AiTool::OpenCode) {
        // Count ses_*.json files directly in the projects_dir (which is storage/session/global/)
        let session_count = count_sessions_for_tool(&projects_dir, ai_tool);

        if session_count > 0 {
            tracing::info!("Found {} OpenCode sessions in global directory", session_count);

            // Extract working directory from the most recent session file
            let working_dir = extract_opencode_working_directory(&projects_dir)
                .unwrap_or_else(|| "OpenCode".to_string());

            // Extract project name from working directory (last segment)
            let project_name = std::path::Path::new(&working_dir)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("OpenCode")
                .to_string();

            let last_updated = get_project_last_updated(&projects_dir, ai_tool);
            projects.insert(
                "global".to_string(),
                ProjectInfo {
                    name: project_name,
                    path: working_dir,
                    encoded_name: "global".to_string(),
                    session_count,
                    ai_tool: ai_tool.clone(),
                    last_updated,
                },
            );
        }

        return Ok(projects.into_values().collect());
    }

    // Fallback: scan filesystem for additional projects
    tracing::info!("Scanning filesystem for additional projects for {:?}", ai_tool);

    let entries = match fs::read_dir(&projects_dir) {
        Ok(entries) => entries,
        Err(e) => {
            tracing::warn!("Failed to read projects directory for {:?}: {}", ai_tool, e);
            return Ok(projects.into_values().collect());
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        tracing::info!("Processing entry: {}", path.display());
        if !path.is_dir() {
            tracing::info!("Skipping non-directory: {}", path.display());
            continue;
        }

        let encoded_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        tracing::info!("Processing directory: {} (encoded: {})", path.display(), encoded_name);

        tracing::debug!("Found directory for {:?}: {}", ai_tool, encoded_name);

        // For Gemini, skip non-hash directories (like 'bin')
        // Gemini uses hash directories with 32+ characters
        if matches!(ai_tool, crate::domain::models::AiTool::Gemini)
            && (encoded_name.len() < 10 || !encoded_name.chars().all(|c| c.is_ascii_hexdigit())) {
                logging::log_gemini_dir_filter(&encoded_name, "not a valid hash directory", true);
                continue;
            }

        // Use tool-specific decode for project paths
        let (project_path, project_name) = match ai_tool {
            crate::domain::models::AiTool::ClaudeCode => {
                // Claude uses dash-separated paths
                let (path, name) = smart_decode_project_path(&encoded_name);
                tracing::debug!("Smart decoded {} -> path: {}, name: {}", encoded_name, path, name);
                (path, name)
            }
            crate::domain::models::AiTool::Gemini => {
                // Gemini uses SHA256 hash of the path
                let (path, name) = decode_gemini_hash(&encoded_name);
                tracing::debug!("Gemini hash {} -> path: {}, name: {}", encoded_name, path, name);
                (path, name)
            }
            _ => {
                // For other tools (OpenCode), use standard decode
                match decode_project_path(&encoded_name) {
                    Ok(p) => {
                        let name = PathBuf::from(&p)
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        tracing::debug!("Decoded {} to {}", encoded_name, p);
                        (p, name)
                    },
                    Err(e) => {
                        tracing::warn!("Failed to decode project directory {}: {}", encoded_name, e);
                        continue;
                    },
                }
            }
        };

        // Check if we already have this project from config
        if projects.contains_key(&encoded_name) {
            tracing::debug!("Project {} already known from config", encoded_name);
            continue;
        }

        // Count sessions using tool-specific logic
        let session_count = count_sessions_for_tool(&path, ai_tool);

        // Skip projects with 0 sessions (empty directories)
        if session_count == 0 {
            logging::log_empty_project_skip(&tool_name, &project_name);
            continue;
        }

        // Log session count
        logging::log_session_count(&tool_name, &project_name, session_count);

        tracing::debug!("Found {} sessions for project {}", session_count, project_name);

        let last_updated = get_project_last_updated(&path, ai_tool);
        projects.insert(
            encoded_name.clone(),
            ProjectInfo {
                name: project_name,
                path: project_path,
                encoded_name,
                session_count,
                ai_tool: ai_tool.clone(),
                last_updated,
            },
        );
    }

    let result: Vec<ProjectInfo> = projects.into_values().collect();
    logging::log_project_discovery_result(&tool_name, result.len(), &projects_dir);
    Ok(result)
}

/// List all projects from ~/.claude/projects/ (legacy function for compatibility)
pub fn list_claude_projects() -> Result<Vec<ProjectInfo>, String> {
    scan_projects_for_tool(&crate::domain::models::AiTool::ClaudeCode)
}
/// Parse a JSONL file and extract only metadata for listing (lightweight)
/// This is much faster than full parsing for listing many sessions
pub fn parse_history_file_metadata(file_path: &Path) -> Result<ConversationFile, String> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut message_ids: HashSet<String> = HashSet::new();
    let mut start_time: Option<String> = None;
    let mut last_time: Option<String> = None;
    let mut last_message_preview = String::new();
    let mut message_count = 0;
    let mut timestamps_found = 0;
    let mut parse_errors = 0;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let entry: RawHistoryLine = match serde_json::from_str(line) {
            Ok(e) => e,
            Err(e) => {
                parse_errors += 1;
                tracing::debug!("Parse error in {}: {}", file_path.display(), e);
                continue;
            }
        };

        message_count += 1;

        // Debug: Track timestamp extraction (use effective_timestamp to include snapshot.timestamp)
        if entry.effective_timestamp().is_some() {
            timestamps_found += 1;
        }

        // Track message IDs from assistant messages (critical for grouping)
        if let Some(ref inner_msg) = entry.message {
            if inner_msg.role.as_deref() == Some("assistant") {
                if let Some(ref msg_id) = inner_msg.id {
                    message_ids.insert(msg_id.clone());
                }
            }
        }

        // Track timestamps (use effective_timestamp to include snapshot.timestamp for file-history-snapshot entries)
        if let Some(ts) = entry.effective_timestamp() {
            match &start_time {
                None => start_time = Some(ts.clone()),
                Some(existing) if ts < existing => start_time = Some(ts.clone()),
                _ => {}
            }
            match &last_time {
                None => last_time = Some(ts.clone()),
                Some(existing) if ts > existing => last_time = Some(ts.clone()),
                _ => {}
            }
        }

        // Extract last message preview (from assistant messages)
        if let Some(ref inner_msg) = entry.message {
            if inner_msg.role.as_deref() == Some("assistant") {
                match &inner_msg.content {
                    MessageContent::Text(text) => {
                        last_message_preview = text.chars().take(100).collect();
                    }
                    MessageContent::Blocks(blocks) => {
                        for block in blocks {
                            if let InnerContentBlock::Text { text } = block {
                                last_message_preview = text.chars().take(100).collect();
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Extract session ID from file name
    let session_id = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Use file modification time as fallback instead of Utc::now()
    // This ensures files with no timestamps (like summary-only files) still show correct dates
    let file_mtime_fallback = std::fs::metadata(file_path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    tracing::debug!(
        "Parsed {}: {} messages, {} timestamps found, {} parse errors, start={:?}, last={:?}",
        session_id,
        message_count,
        timestamps_found,
        parse_errors,
        start_time,
        last_time
    );

    Ok(ConversationFile {
        session_id,
        file_path: file_path.to_string_lossy().to_string(),
        message_ids,
        start_time: start_time.unwrap_or_else(|| file_mtime_fallback.clone()),
        last_time: last_time.unwrap_or(file_mtime_fallback),
        message_count,
        last_message_preview: if last_message_preview.is_empty() {
            "No preview available".to_string()
        } else {
            last_message_preview
        },
    })
}

/// List conversation summaries for a project (with deduplication)
/// This is the recommended function for listing - it removes duplicates from continued sessions
/// Supports all AI tools: Claude (.jsonl), OpenCode (ses_*.json), Gemini (chats/session-*.json)
pub fn list_project_summaries(encoded_name: &str) -> Result<Vec<ConversationSummary>, String> {
    // Try all AI tool directories to find the project
    let ai_tools = vec![
        crate::domain::models::AiTool::ClaudeCode,
        crate::domain::models::AiTool::OpenCode,
        crate::domain::models::AiTool::Gemini,
    ];

    for ai_tool in &ai_tools {
        let projects_dir = match get_projects_dir(ai_tool) {
            Ok(dir) => dir,
            Err(_) => continue,
        };

        // Special case for OpenCode: "global" is the projects_dir itself, not a subdirectory
        let project_dir = if matches!(ai_tool, crate::domain::models::AiTool::OpenCode) && encoded_name == "global" {
            projects_dir.clone()
        } else {
            projects_dir.join(encoded_name)
        };

        if !project_dir.exists() {
            continue;
        }

        tracing::info!("Found project {} in {:?} directory", encoded_name, ai_tool);

        // Use specialized parsers for each AI tool
        let conversation_files = match ai_tool {
            crate::domain::models::AiTool::ClaudeCode => {
                super::parsers::parse_claude_summaries(&project_dir)?
            }
            crate::domain::models::AiTool::OpenCode => {
                super::parsers::parse_opencode_summaries(&project_dir)?
            }
            crate::domain::models::AiTool::Gemini => {
                super::parsers::parse_gemini_summaries(&project_dir)?
            }
        };

        if !conversation_files.is_empty() {
            tracing::debug!("Before grouping: {} conversation files", conversation_files.len());
            // Apply grouping algorithm to remove duplicates (mainly for Claude)
            let summaries = group_conversations(conversation_files);
            tracing::debug!("After grouping: {} summaries", summaries.len());
            return Ok(summaries);
        }
    }

    // Project not found in any directory
    Ok(Vec::new())
}

/// List project summaries for a specific AI tool
/// tool_slug: Optional filter ("claude", "opencode", "gemini") - if None, searches all tools
pub fn list_project_summaries_for_tool(encoded_name: &str, tool_slug: Option<&str>) -> Result<Vec<ConversationSummary>, String> {
    // Convert tool slug to AiTool enum
    let target_tool = tool_slug.and_then(|slug| match slug {
        "claude" => Some(crate::domain::models::AiTool::ClaudeCode),
        "opencode" => Some(crate::domain::models::AiTool::OpenCode),
        "gemini" => Some(crate::domain::models::AiTool::Gemini),
        _ => None,
    });

    // If specific tool requested, only search that one
    let ai_tools: Vec<crate::domain::models::AiTool> = match target_tool {
        Some(tool) => vec![tool],
        None => vec![
            crate::domain::models::AiTool::ClaudeCode,
            crate::domain::models::AiTool::OpenCode,
            crate::domain::models::AiTool::Gemini,
        ],
    };

    // Collect all conversation files from requested tool(s)
    let mut all_conversation_files = Vec::new();

    for ai_tool in &ai_tools {
        let projects_dir = match get_projects_dir(ai_tool) {
            Ok(dir) => dir,
            Err(_) => continue,
        };

        // Special case for OpenCode: "global" is the projects_dir itself, not a subdirectory
        let project_dir = if matches!(ai_tool, crate::domain::models::AiTool::OpenCode) && encoded_name == "global" {
            projects_dir.clone()
        } else {
            projects_dir.join(encoded_name)
        };

        if !project_dir.exists() {
            tracing::debug!("Project dir not found: {:?} (tool: {:?})", project_dir, ai_tool);
            continue;
        }

        tracing::info!("Found project {} in {:?} directory (tool filter: {:?})", encoded_name, ai_tool, tool_slug);

        // Use specialized parsers for each AI tool
        let conversation_files = match ai_tool {
            crate::domain::models::AiTool::ClaudeCode => {
                super::parsers::parse_claude_summaries(&project_dir)?
            }
            crate::domain::models::AiTool::OpenCode => {
                super::parsers::parse_opencode_summaries(&project_dir)?
            }
            crate::domain::models::AiTool::Gemini => {
                super::parsers::parse_gemini_summaries(&project_dir)?
            }
        };

        tracing::info!("Tool {:?} returned {} conversation files for {}", ai_tool, conversation_files.len(), encoded_name);
        all_conversation_files.extend(conversation_files);
    }

    if !all_conversation_files.is_empty() {
        tracing::debug!("Before grouping: {} total conversation files", all_conversation_files.len());
        let summaries = group_conversations(all_conversation_files);
        tracing::debug!("After grouping: {} summaries", summaries.len());
        return Ok(summaries);
    }

    // Project not found
    Ok(Vec::new())
}

/// List all conversation histories for a specific project (full parsing)
/// Note: For listing purposes, prefer `list_project_summaries` which is faster and deduplicates
pub fn list_project_histories(encoded_name: &str) -> Result<Vec<ConversationHistory>, String> {
    let projects_dir = get_claude_projects_dir()?;
    let project_dir = projects_dir.join(encoded_name);

    if !project_dir.exists() {
        return Ok(Vec::new());
    }

    // First get the deduplicated summaries to know which sessions to load
    let summaries = list_project_summaries(encoded_name)?;
    let valid_session_ids: HashSet<String> = summaries.iter()
        .map(|s| s.session_id.clone())
        .collect();

    let mut histories = Vec::new();

    let entries = fs::read_dir(&project_dir)
        .map_err(|e| format!("Failed to read project directory: {}", e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }

        // Only load sessions that passed the deduplication filter
        let session_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        if !valid_session_ids.contains(session_id) {
            tracing::debug!("Skipping duplicate session: {}", session_id);
            continue;
        }

        match parse_jsonl_file(&path) {
            Ok(history) => histories.push(history),
            Err(e) => tracing::warn!("Failed to parse {}: {}", path.display(), e),
        }
    }

    // Sort by updated_at descending (newest first)
    histories.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(histories)
}

/// Load a specific conversation by session ID with timestamp restoration
/// Supports all AI tools: Claude (.jsonl), OpenCode (ses_*.json), Gemini (chats/session-*.json)
pub fn load_conversation(encoded_name: &str, session_id: &str) -> Result<ConversationHistory, String> {
    tracing::debug!("load_conversation called: encoded_name='{}', session_id='{}'", encoded_name, session_id);

    // Try all AI tool directories to find the session
    let ai_tools = vec![
        crate::domain::models::AiTool::ClaudeCode,
        crate::domain::models::AiTool::OpenCode,
        crate::domain::models::AiTool::Gemini,
    ];

    for ai_tool in &ai_tools {
        let projects_dir = match get_projects_dir(ai_tool) {
            Ok(dir) => dir,
            Err(_) => continue,
        };

        // Special case for OpenCode: "global" is the projects_dir itself, not a subdirectory
        let project_dir = if matches!(ai_tool, crate::domain::models::AiTool::OpenCode) && encoded_name == "global" {
            projects_dir.clone()
        } else {
            projects_dir.join(encoded_name)
        };

        tracing::debug!("Trying {:?}: project_dir={}, exists={}", ai_tool, project_dir.display(), project_dir.exists());

        if !project_dir.exists() {
            continue;
        }

        // Different file patterns per AI tool
        let file_path = match ai_tool {
            crate::domain::models::AiTool::ClaudeCode => {
                project_dir.join(format!("{}.jsonl", session_id))
            }
            crate::domain::models::AiTool::OpenCode => {
                project_dir.join(format!("ses_{}.json", session_id))
            }
            crate::domain::models::AiTool::Gemini => {
                project_dir.join("chats").join(format!("session-{}.json", session_id))
            }
        };

        tracing::debug!("Checking file: {}, exists={}", file_path.display(), file_path.exists());

        if file_path.exists() {
            tracing::info!("Found session {} in {:?} directory at {}", session_id, ai_tool, file_path.display());

            // Claude uses JSONL, OpenCode/Gemini use JSON
            return match ai_tool {
                crate::domain::models::AiTool::ClaudeCode => parse_jsonl_file(&file_path),
                crate::domain::models::AiTool::OpenCode | crate::domain::models::AiTool::Gemini => {
                    // For OpenCode/Gemini, parse JSON and convert to ConversationHistory
                    parse_json_session_file(&file_path, session_id, ai_tool)
                }
            };
        }
    }

    Err(format!("Session not found: {}", session_id))
}

/// Parse OpenCode or Gemini JSON session file
fn parse_json_session_file(file_path: &Path, session_id: &str, ai_tool: &crate::domain::models::AiTool) -> Result<ConversationHistory, String> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let json: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    let mut messages = Vec::new();
    let now = chrono::Utc::now();

    // OpenCode stores messages in a SEPARATE directory: ~/.local/share/opencode/storage/message/ses_{id}/msg_*.json
    // The session file only contains metadata (title, time, etc.)
    if matches!(ai_tool, crate::domain::models::AiTool::OpenCode) {
        messages = super::parsers::opencode::load_opencode_messages(session_id)?;
    } else {
        // Gemini format: { "messages": [...] } or root array
        // Note: Gemini uses "type" field ("user"/"gemini") instead of "role"
        let messages_array = json.get("messages")
            .and_then(|m| m.as_array())
            .or_else(|| json.as_array());

        if let Some(msgs) = messages_array {
            for msg in msgs {
                // Gemini uses "type" field, but also check "role" for compatibility
                let role = msg.get("type")
                    .and_then(|r| r.as_str())
                    .or_else(|| msg.get("role").and_then(|r| r.as_str()))
                    .map(|r| {
                        // Normalize "gemini" to "assistant" for consistent display
                        if r == "gemini" { "assistant" } else { r }
                    })
                    .unwrap_or("unknown")
                    .to_string();

                let content_text = msg.get("content")
                    .map(|c| {
                        if let Some(s) = c.as_str() {
                            s.to_string()
                        } else if let Some(arr) = c.as_array() {
                            // Handle content blocks array
                            arr.iter()
                                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                                .collect::<Vec<_>>()
                                .join("\n")
                        } else {
                            c.to_string()
                        }
                    })
                    .unwrap_or_default();

                let timestamp = msg.get("timestamp")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());

                messages.push(Message {
                    role,
                    content: vec![ContentBlock::Text { text: content_text }],
                    timestamp,
                });
            }
        }
    }

    let tool_name = match ai_tool {
        crate::domain::models::AiTool::ClaudeCode => "Claude",
        crate::domain::models::AiTool::OpenCode => "OpenCode",
        crate::domain::models::AiTool::Gemini => "Gemini",
    };

    Ok(ConversationHistory {
        session_id: session_id.to_string(),
        project_path: file_path.parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        project_name: tool_name.to_string(),
        messages: messages.clone(),
        created_at: now,
        updated_at: now,
        message_count: messages.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_jsonl() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_empty.jsonl");
        fs::write(&test_file, "").unwrap();

        let result = parse_jsonl_file(&test_file);
        assert!(result.is_ok());
        let history = result.unwrap();
        assert_eq!(history.messages.len(), 0);

        fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_parse_real_jsonl_metadata() {
        // Test with a sample Claude JSONL content
        let sample_content = r#"{"type":"queue-operation","operation":"enqueue","timestamp":"2025-11-25T10:02:50.253Z","content":"/mcp","sessionId":"01422bfa-5189-4a64-8b5a-c7fd212d5362"}
{"type":"queue-operation","operation":"dequeue","timestamp":"2025-11-25T10:02:50.254Z","sessionId":"01422bfa-5189-4a64-8b5a-c7fd212d5362"}
{"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/Users/toto/Desktop/Code","sessionId":"01422bfa-5189-4a64-8b5a-c7fd212d5362","version":"2.0.50","gitBranch":"","type":"user","message":{"role":"user","content":"test message"},"uuid":"c143ba4a-0d4e-40da-8179-85e52adfaa4b","timestamp":"2025-11-25T10:02:50.258Z"}
{"parentUuid":"c143ba4a","isSidechain":false,"userType":"external","cwd":"/Users/toto/Desktop/Code","sessionId":"01422bfa-5189-4a64-8b5a-c7fd212d5362","version":"2.0.50","gitBranch":"","message":{"model":"claude-sonnet-4-5-20250929","id":"msg_01EekSVKmRj65kYVZRpiJ3oH","type":"message","role":"assistant","content":[{"type":"text","text":"Hello! How can I help you today?"}],"stop_reason":"end_turn"},"type":"assistant","uuid":"ea86c813-e051-42e6-9bbe-a42ec0417038","timestamp":"2025-11-25T10:02:56.207Z"}"#;

        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_real_format.jsonl");
        fs::write(&test_file, sample_content).unwrap();

        let result = parse_history_file_metadata(&test_file);

        // Print debug info
        match &result {
            Ok(conv_file) => {
                println!("\n=== DEBUG parse_history_file_metadata ===");
                println!("session_id: {}", conv_file.session_id);
                println!("message_count: {}", conv_file.message_count);
                println!("start_time: {}", conv_file.start_time);
                println!("last_time: {}", conv_file.last_time);
                println!("preview: {}", conv_file.last_message_preview);
                println!("===========================================\n");
            }
            Err(e) => {
                println!("\n=== DEBUG ERROR: {} ===\n", e);
            }
        }

        assert!(result.is_ok(), "parse_history_file_metadata should succeed");
        let conv_file = result.unwrap();

        // Verify timestamps were extracted
        assert!(!conv_file.start_time.contains("2025-11-26"),
            "start_time should be from file content, not current time. Got: {}", conv_file.start_time);
        assert!(conv_file.start_time.contains("2025-11-25"),
            "start_time should be 2025-11-25. Got: {}", conv_file.start_time);
        assert!(conv_file.last_time.contains("2025-11-25"),
            "last_time should be 2025-11-25. Got: {}", conv_file.last_time);

        // Cleanup
        fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_raw_history_line_deserialization() {
        // Test that RawHistoryLine correctly deserializes timestamp
        let json_line = r#"{"type":"user","timestamp":"2025-11-25T10:02:50.258Z","sessionId":"test-123","message":{"role":"user","content":"hello"}}"#;

        let result: Result<RawHistoryLine, _> = serde_json::from_str(json_line);

        match &result {
            Ok(entry) => {
                println!("\n=== DEBUG RawHistoryLine deserialization ===");
                println!("timestamp: {:?}", entry.timestamp);
                println!("entry_type: {:?}", entry.entry_type);
                println!("session_id: {:?}", entry.session_id);
                println!("===========================================\n");
            }
            Err(e) => {
                println!("\n=== DEBUG DESER ERROR: {} ===\n", e);
            }
        }

        assert!(result.is_ok(), "Should deserialize: {:?}", result.err());
        let entry = result.unwrap();
        assert!(entry.timestamp.is_some(), "timestamp should be Some");
        assert_eq!(entry.timestamp.unwrap(), "2025-11-25T10:02:50.258Z");
    }

    #[test]
    fn test_list_summaries_real_project() {
        // Test with the real project "-Users-toto-Desktop-Code"
        // This helps debug the 500 error on project pages
        let project_name = "-Users-toto-Desktop-Code";

        println!("\n=== Testing list_project_summaries with real project ===");
        println!("Project: {}", project_name);

        let result = list_project_summaries(project_name);

        match &result {
            Ok(summaries) => {
                println!("SUCCESS: {} summaries found", summaries.len());
                for (i, s) in summaries.iter().take(5).enumerate() {
                    let preview_short = &s.last_message_preview[..50.min(s.last_message_preview.len())];
                    println!("  {}. session_id: {}", i + 1, &s.session_id[..8.min(s.session_id.len())]);
                    println!("     message_count: {}", s.message_count);
                    println!("     last_time: '{}'", s.last_time);
                    println!("     preview: '{}'", preview_short);
                }
            }
            Err(e) => {
                println!("ERROR: {}", e);
            }
        }

        assert!(result.is_ok(), "list_project_summaries should succeed: {:?}", result.err());
        let summaries = result.unwrap();
        assert!(summaries.len() > 0, "Should find at least one session");

        // Verify timestamps are not empty
        for s in &summaries {
            assert!(!s.last_time.is_empty(), "last_time should not be empty for session {}", s.session_id);
        }

        println!("===========================================\n");
    }
}
