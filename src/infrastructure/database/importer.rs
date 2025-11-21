//! JSONL to SurrealDB importer
//!
//! Reads Claude history files and imports them into SurrealDB.

use std::collections::HashSet;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use surrealdb::engine::local::Db;
use surrealdb::sql::Thing;
use surrealdb::Surreal;

use crate::history::parser::{parse_history_file_metadata, RawHistoryLine, InnerMessage, MessageContent};
use crate::history::path_utils::{get_claude_projects_dir, list_project_folders, decode_project_path};
use crate::history::grouping::group_conversations;

use super::repositories::{
    project_repo::{CreateProject, ProjectRepository},
    conversation_repo::{CreateConversation, ConversationRepository},
    message_repo::{CreateMessage, MessageRepository},
};

/// Import result statistics
#[derive(Debug, Default)]
pub struct ImportStats {
    pub projects_imported: usize,
    pub conversations_imported: usize,
    pub messages_imported: usize,
    pub errors: Vec<String>,
}

/// Import all Claude history into SurrealDB
pub async fn import_all_history(db: &Surreal<Db>) -> Result<ImportStats, String> {
    let mut stats = ImportStats::default();

    // Get Claude projects directory
    let projects_dir = get_claude_projects_dir()?;

    // List all project folders
    let project_folders = list_project_folders(&projects_dir)?;

    for folder in project_folders {
        match import_project(db, &folder).await {
            Ok((_, conv_count, msg_count)) => {
                stats.projects_imported += 1;
                stats.conversations_imported += conv_count;
                stats.messages_imported += msg_count;
            }
            Err(e) => {
                stats.errors.push(format!("Project {}: {}", folder.display(), e));
            }
        }
    }

    Ok(stats)
}

/// Import a single project
async fn import_project(
    db: &Surreal<Db>,
    project_path: &PathBuf,
) -> Result<(usize, usize, usize), String> {
    let folder_name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid project folder name")?;

    // Decode project name
    let decoded_name = decode_project_path(folder_name)
        .unwrap_or_else(|_| folder_name.to_string());

    // List session files
    let session_files = list_session_files(project_path)?;

    if session_files.is_empty() {
        return Ok((0, 0, 0));
    }

    // Parse all session files to get metadata for grouping
    let mut conversation_files = Vec::new();
    for file_path in &session_files {
        if let Ok(metadata) = parse_history_file_metadata(file_path) {
            conversation_files.push(metadata);
        }
    }

    // Group conversations (deduplicate by message.id sets)
    let grouped = group_conversations(conversation_files.clone());

    // Create or update project in DB
    let project = ProjectRepository::upsert(db, CreateProject {
        name: decoded_name.clone(),
        path: decoded_name.clone(),
        encoded_name: folder_name.to_string(),
        ai_tool: "claude_code".to_string(),
        session_count: grouped.len() as i32,
    }).await?;

    let project_id = project.id.ok_or("Project created without ID")?;

    // Import each conversation (only the "final" ones after deduplication)
    let mut total_conversations = 0;
    let mut total_messages = 0;

    // Create a map from session_id to file_path for quick lookup
    let file_map: std::collections::HashMap<String, String> = conversation_files
        .iter()
        .map(|cf| (cf.session_id.clone(), cf.file_path.clone()))
        .collect();

    for summary in grouped {
        // Find the file path for this session
        let file_path = match file_map.get(&summary.session_id) {
            Some(path) => PathBuf::from(path),
            None => continue,
        };

        match import_conversation(db, &project_id, &summary.session_id, &file_path).await {
            Ok(msg_count) => {
                total_conversations += 1;
                total_messages += msg_count;
            }
            Err(e) => {
                tracing::warn!("Failed to import conversation {}: {}", summary.session_id, e);
            }
        }
    }

    Ok((1, total_conversations, total_messages))
}

/// Import a single conversation
async fn import_conversation(
    db: &Surreal<Db>,
    project_id: &Thing,
    session_id: &str,
    file_path: &PathBuf,
) -> Result<usize, String> {
    // Read and parse the file
    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    let mut raw_messages = Vec::new();
    let mut start_time: Option<String> = None;
    let mut end_time: Option<String> = None;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if let Ok(raw) = serde_json::from_str::<RawHistoryLine>(line) {
            // Track timestamps
            if let Some(ts) = &raw.timestamp {
                if start_time.is_none() || ts < start_time.as_ref().unwrap() {
                    start_time = Some(ts.clone());
                }
                if end_time.is_none() || ts > end_time.as_ref().unwrap() {
                    end_time = Some(ts.clone());
                }
            }

            raw_messages.push(raw);
        }
    }

    if raw_messages.is_empty() {
        return Ok(0);
    }

    // Create preview from last message
    let last_preview = raw_messages
        .last()
        .and_then(|m| extract_preview_text(&m.message))
        .unwrap_or_default();

    // Parse timestamps
    let start_dt = parse_timestamp(&start_time).unwrap_or_else(Utc::now);
    let end_dt = parse_timestamp(&end_time).unwrap_or_else(Utc::now);

    // Create/update conversation in DB
    let conversation = ConversationRepository::upsert(db, CreateConversation {
        project_id: project_id.clone(),
        session_id: session_id.to_string(),
        source_files: vec![file_path.to_string_lossy().to_string()],
        message_count: raw_messages.len() as i32,
        start_time: start_dt,
        end_time: end_dt,
        last_preview,
    }).await?;

    let conversation_id = conversation.id.ok_or("Conversation created without ID")?;

    // Check which messages already exist
    let existing_msg_ids = get_existing_message_ids(db, &conversation_id).await?;

    // Create messages for new ones only
    let mut new_messages = Vec::new();
    for raw in raw_messages {
        let msg_id = raw.message.as_ref()
            .and_then(|m| m.id.clone());

        // Skip if message already exists
        if let Some(ref id) = msg_id {
            if existing_msg_ids.contains(id) {
                continue;
            }
        }

        // Get role from inner message
        let role = raw.message.as_ref()
            .and_then(|m| m.role.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Convert message to JSON Value
        let content_json = match &raw.message {
            Some(msg) => serde_json::to_value(msg).unwrap_or(serde_json::Value::Null),
            None => serde_json::Value::Null,
        };

        let timestamp = parse_timestamp(&raw.timestamp).unwrap_or_else(Utc::now);

        new_messages.push(CreateMessage {
            conversation_id: conversation_id.clone(),
            message_id: msg_id,
            role,
            content: content_json,
            timestamp,
            uuid: raw.uuid,
            parent_uuid: raw.parent_uuid,
            is_sidechain: raw.is_sidechain.unwrap_or(false),
        });
    }

    let imported_count = new_messages.len();

    if !new_messages.is_empty() {
        MessageRepository::create_batch(db, new_messages).await?;
    }

    Ok(imported_count)
}

/// Parse ISO 8601 timestamp string to DateTime
fn parse_timestamp(ts: &Option<String>) -> Option<DateTime<Utc>> {
    ts.as_ref().and_then(|s| {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .ok()
    })
}

/// List JSONL session files in a project folder
fn list_session_files(project_path: &PathBuf) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(project_path)
        .map_err(|e| format!("Failed to read project directory: {}", e))?;

    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }

    Ok(files)
}

/// Get existing message IDs for a conversation
async fn get_existing_message_ids(
    db: &Surreal<Db>,
    conversation_id: &Thing,
) -> Result<HashSet<String>, String> {
    let messages = MessageRepository::find_by_conversation(db, conversation_id).await?;

    Ok(messages
        .into_iter()
        .filter_map(|m| m.message_id)
        .collect())
}

/// Extract preview text from InnerMessage
fn extract_preview_text(message: &Option<InnerMessage>) -> Option<String> {
    let msg = message.as_ref()?;

    match &msg.content {
        MessageContent::Text(text) => Some(text.chars().take(100).collect()),
        MessageContent::Blocks(blocks) => {
            for block in blocks {
                match block {
                    crate::history::parser::InnerContentBlock::Text { text } => {
                        return Some(text.chars().take(100).collect());
                    }
                    _ => continue,
                }
            }
            None
        }
    }
}

/// Sync a single project (incremental update)
pub async fn sync_project(
    db: &Surreal<Db>,
    encoded_name: &str,
) -> Result<ImportStats, String> {
    let projects_dir = get_claude_projects_dir()?;
    let project_path = projects_dir.join(encoded_name);

    if !project_path.exists() {
        return Err(format!("Project folder not found: {}", encoded_name));
    }

    let mut stats = ImportStats::default();

    match import_project(db, &project_path).await {
        Ok((_, conv_count, msg_count)) => {
            stats.projects_imported = 1;
            stats.conversations_imported = conv_count;
            stats.messages_imported = msg_count;
        }
        Err(e) => {
            stats.errors.push(e);
        }
    }

    Ok(stats)
}
