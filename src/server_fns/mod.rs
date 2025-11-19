//! Server functions for Dioxus Fullstack
//! These functions run on the server and are callable from the client

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::domain::models::AiTool;

// ============================================================
// Session Cache (server-side only)
// ============================================================
mod cache {
    use dashmap::DashMap;
    use once_cell::sync::Lazy;
    use std::time::{Duration, Instant};

    /// Cached conversation with timestamp for TTL
    pub struct CachedConversation {
        pub conversation: crate::domain::models::Conversation,
        pub cached_at: Instant,
    }

    /// Cache key: (encoded_project_name, session_id)
    pub type CacheKey = (String, String);

    /// Global cache for conversations (thread-safe)
    /// TTL: 5 minutes - after that, re-read from disk to get updates
    pub static CONVERSATION_CACHE: Lazy<DashMap<CacheKey, CachedConversation>> =
        Lazy::new(DashMap::new);

    /// Cache TTL: 5 minutes
    pub const CACHE_TTL: Duration = Duration::from_secs(300);

    /// Get from cache if not expired
    #[allow(dead_code)]
    pub fn get_cached(
        encoded_name: &str,
        session_id: &str,
    ) -> Option<crate::domain::models::Conversation> {
        let key = (encoded_name.to_string(), session_id.to_string());
        if let Some(entry) = CONVERSATION_CACHE.get(&key) {
            if entry.cached_at.elapsed() < CACHE_TTL {
                return Some(entry.conversation.clone());
            } else {
                // Expired, remove from cache
                drop(entry);
                CONVERSATION_CACHE.remove(&key);
            }
        }
        None
    }

    /// Insert into cache
    #[allow(dead_code)]
    pub fn set_cached(
        encoded_name: &str,
        session_id: &str,
        conversation: crate::domain::models::Conversation,
    ) {
        let key = (encoded_name.to_string(), session_id.to_string());
        CONVERSATION_CACHE.insert(
            key,
            CachedConversation {
                conversation,
                cached_at: Instant::now(),
            },
        );
    }
}

/// Response type for project listing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectResponse {
    pub name: String,
    pub path: String,
    pub session_count: i32,
    pub ai_tool: AiTool,
    pub encoded_name: String,
    pub last_updated: String,
}

/// Response type for conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryResponse {
    pub session_id: String,
    pub project_path: String,
    pub project_name: String,
    pub messages: Vec<MessageResponse>,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: usize,
}

/// Response type for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub role: String,
    pub content: Vec<ContentBlockResponse>,
    pub timestamp: Option<String>,
}

/// Response type for content blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlockResponse {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult { content: String },
}

/// Lightweight session summary response (no messages - FAST)
/// Used for listing sessions without loading all message content
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSummaryResponse {
    pub session_id: String,
    pub message_count: usize,
    pub preview: String,
    pub updated_at: String,
}

/// Get a single project by encoded_name
/// Returns the project with its real filesystem path
#[server]
pub async fn get_project(encoded_name: String, tool_slug: Option<String>) -> Result<Option<ProjectResponse>, ServerFnError> {
    use crate::history::list_projects;
    use crate::domain::models::AiTool;

    let projects = list_projects().map_err(|e| ServerFnError::new(e))?;

    // Filter by tool if provided
    let target_tool = tool_slug.as_ref().and_then(|slug| match slug.as_str() {
        "claude" => Some(AiTool::ClaudeCode),
        "opencode" => Some(AiTool::OpenCode),
        "gemini" => Some(AiTool::Gemini),
        _ => None,
    });

    let project = projects
        .into_iter()
        .find(|p| {
            p.encoded_name == encoded_name
                && target_tool.as_ref().map(|t| &p.ai_tool == t).unwrap_or(true)
        })
        .map(|p| ProjectResponse {
            name: p.name,
            path: p.path,
            session_count: p.session_count as i32,
            ai_tool: p.ai_tool,
            encoded_name: p.encoded_name,
            last_updated: p.last_updated.to_rfc3339(),
        });

    Ok(project)
}

/// List all projects from Claude history
/// tool_filter: Optional tool slug ("claude", "opencode", "gemini") to filter projects
#[server]
pub async fn get_projects(search: Option<String>, tool_filter: Option<String>) -> Result<Vec<ProjectResponse>, ServerFnError> {
    tracing::info!("get_projects called with search: {:?}, tool_filter: {:?}", search, tool_filter);
    use crate::history::list_projects;
    use crate::domain::models::AiTool;

    let projects = list_projects().map_err(|e| ServerFnError::new(e))?;

    let mut results: Vec<ProjectResponse> = projects
        .into_iter()
        .map(|p| ProjectResponse {
            name: p.name,
            path: p.path,
            session_count: p.session_count as i32,
            ai_tool: p.ai_tool,
            encoded_name: p.encoded_name,
            last_updated: p.last_updated.to_rfc3339(),
        })
        .collect();

    // Filter by AI tool if provided
    if let Some(tool_slug) = tool_filter {
        let target_tool = match tool_slug.as_str() {
            "claude" => Some(AiTool::ClaudeCode),
            "opencode" => Some(AiTool::OpenCode),
            "gemini" => Some(AiTool::Gemini),
            _ => None,
        };
        if let Some(tool) = target_tool {
            results.retain(|p| p.ai_tool == tool);
        }
    }

    // Filter by search term if provided
    if let Some(query) = search {
        let query_lower = query.to_lowercase();
        results.retain(|p| {
            p.name.to_lowercase().contains(&query_lower)
                || p.path.to_lowercase().contains(&query_lower)
        });
    }

    // Sort by last_updated descending (most recent first)
    results.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));

    Ok(results)
}

/// OPTIMIZED: List session summaries (metadata only, no messages)
/// This is 10-100x faster than get_histories() for listing sessions
/// tool_slug: Optional tool filter ("claude", "opencode", "gemini")
#[server]
pub async fn get_sessions_summaries(
    encoded_name: String,
    tool_slug: Option<String>,
) -> Result<Vec<SessionSummaryResponse>, ServerFnError> {
    use crate::history::list_project_summaries_for_tool;

    println!(">>> DEBUG get_sessions_summaries called: encoded_name={}, tool_slug={:?}", encoded_name, tool_slug);
    tracing::info!("get_sessions_summaries called: encoded_name={}, tool_slug={:?}", encoded_name, tool_slug);

    let summaries = list_project_summaries_for_tool(&encoded_name, tool_slug.as_deref())
        .map_err(|e| ServerFnError::new(e))?;

    tracing::info!("get_sessions_summaries: found {} summaries", summaries.len());

    let mut results: Vec<SessionSummaryResponse> = summaries
        .into_iter()
        .map(|s| SessionSummaryResponse {
            session_id: s.session_id,
            message_count: s.message_count,
            preview: s.last_message_preview,
            updated_at: s.last_time,
        })
        .collect();

    // Sort by updated_at descending (most recent first)
    results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(results)
}

/// List conversation histories for a specific project
#[server]
pub async fn get_histories(encoded_name: String) -> Result<Vec<HistoryResponse>, ServerFnError> {
    use crate::history::{list_project_histories, ContentBlock};

    let histories =
        list_project_histories(&encoded_name).map_err(|e| ServerFnError::new(e))?;

    let results: Vec<HistoryResponse> = histories
        .into_iter()
        .map(|h| HistoryResponse {
            session_id: h.session_id,
            project_path: h.project_path,
            project_name: h.project_name,
            messages: h
                .messages
                .into_iter()
                .map(|m| MessageResponse {
                    role: m.role,
                    content: m
                        .content
                        .into_iter()
                        .map(|c| match c {
                            ContentBlock::Text { text } => ContentBlockResponse::Text { text },
                            ContentBlock::ToolUse {
                                tool_type: _,
                                name,
                                input,
                            } => ContentBlockResponse::ToolUse { name, input },
                            ContentBlock::ToolResult {
                                result_type: _,
                                content,
                            } => ContentBlockResponse::ToolResult { content },
                        })
                        .collect(),
                    timestamp: m.timestamp,
                })
                .collect(),
            created_at: h.created_at.to_rfc3339(),
            updated_at: h.updated_at.to_rfc3339(),
            message_count: h.message_count,
        })
        .collect();

    Ok(results)
}

/// Get a single conversation history by session ID
#[server]
pub async fn get_history(
    encoded_name: String,
    session_id: String,
) -> Result<Option<HistoryResponse>, ServerFnError> {
    let histories = get_histories(encoded_name).await?;

    Ok(histories.into_iter().find(|h| h.session_id == session_id))
}

/// Delete a session file from disk
/// Supports all AI tools: Claude (.jsonl), OpenCode (ses_*.json), Gemini (chats/session-*.json)
#[server]
pub async fn delete_session(
    encoded_name: String,
    session_id: String,
) -> Result<bool, ServerFnError> {
    use crate::history::path_utils::get_projects_dir;
    use crate::domain::models::AiTool;
    use std::fs;

    tracing::info!(
        "delete_session called: encoded_name='{}', session_id='{}'",
        encoded_name,
        session_id
    );

    // Try all AI tool directories to find and delete the session
    let ai_tools = vec![AiTool::ClaudeCode, AiTool::OpenCode, AiTool::Gemini];

    for ai_tool in &ai_tools {
        let projects_dir = match get_projects_dir(ai_tool) {
            Ok(dir) => dir,
            Err(_) => continue,
        };

        let project_dir = projects_dir.join(&encoded_name);
        if !project_dir.exists() {
            continue;
        }

        // Different file patterns per AI tool
        let file_path = match ai_tool {
            AiTool::ClaudeCode => project_dir.join(format!("{}.jsonl", session_id)),
            AiTool::OpenCode => project_dir.join(format!("ses_{}.json", session_id)),
            AiTool::Gemini => project_dir.join("chats").join(format!("session-{}.json", session_id)),
        };

        if file_path.exists() {
            tracing::info!("Found session file to delete: {}", file_path.display());

            // Delete the file
            match fs::remove_file(&file_path) {
                Ok(_) => {
                    tracing::info!("Successfully deleted session: {}", session_id);

                    // Invalidate cache for this session
                    let cache_key = (encoded_name.clone(), session_id.clone());
                    cache::CONVERSATION_CACHE.remove(&cache_key);

                    return Ok(true);
                }
                Err(e) => {
                    tracing::error!("Failed to delete session file: {}", e);
                    return Err(ServerFnError::new(format!("Failed to delete session: {}", e)));
                }
            }
        }
    }

    // Session not found
    tracing::warn!("Session not found for deletion: {}", session_id);
    Err(ServerFnError::new(format!("Session not found: {}", session_id)))
}

/// Delete a project (all sessions in the project directory)
#[server]
pub async fn delete_project(encoded_name: String) -> Result<bool, ServerFnError> {
    use crate::history::path_utils::get_projects_dir;
    use crate::domain::models::AiTool;
    use std::fs;

    tracing::info!("delete_project called: encoded_name='{}'", encoded_name);

    // Try all AI tool directories to find and delete the project
    let ai_tools = vec![AiTool::ClaudeCode, AiTool::OpenCode, AiTool::Gemini];
    let mut deleted = false;

    for ai_tool in &ai_tools {
        let projects_dir = match get_projects_dir(ai_tool) {
            Ok(dir) => dir,
            Err(_) => continue,
        };

        let project_dir = projects_dir.join(&encoded_name);
        if !project_dir.exists() {
            continue;
        }

        tracing::info!("Found project directory to delete: {}", project_dir.display());

        // Delete the entire project directory
        match fs::remove_dir_all(&project_dir) {
            Ok(_) => {
                tracing::info!("Successfully deleted project: {}", encoded_name);
                deleted = true;

                // Invalidate all cache entries for this project
                cache::CONVERSATION_CACHE.retain(|key, _| key.0 != encoded_name);
            }
            Err(e) => {
                tracing::error!("Failed to delete project directory: {}", e);
                return Err(ServerFnError::new(format!("Failed to delete project: {}", e)));
            }
        }
    }

    if deleted {
        Ok(true)
    } else {
        tracing::warn!("Project not found for deletion: {}", encoded_name);
        Err(ServerFnError::new(format!("Project not found: {}", encoded_name)))
    }
}

/// Get conversation for viewing (converts to domain Conversation type)
/// OPTIMIZED: Uses load_conversation() to parse ONLY the requested session file
/// instead of parsing ALL session files in the project
/// CACHED: Returns cached result if available (TTL 5 min)
#[server]
pub async fn get_conversation(
    encoded_name: String,
    session_id: String,
) -> Result<Option<crate::domain::models::Conversation>, ServerFnError> {
    use crate::domain::models::{Conversation, Message};
    use crate::history::{load_conversation, ContentBlock};
    use chrono::{DateTime, Utc};

    tracing::debug!(
        "get_conversation called: encoded_name='{}', session_id='{}'",
        encoded_name,
        session_id
    );

    // OPTIMIZATION 1: Check cache first (instant return if cached)
    if let Some(cached) = cache::get_cached(&encoded_name, &session_id) {
        tracing::info!("Cache hit for session: {}", session_id);
        return Ok(Some(cached));
    }

    // OPTIMIZATION 2: Load only the requested session file directly
    // Previously: get_history() -> get_histories() -> parse ALL files -> find one
    // Now: load_conversation() -> parse ONLY one file
    tracing::info!("Loading conversation from disk...");
    let history = match load_conversation(&encoded_name, &session_id) {
        Ok(h) => {
            tracing::info!("Conversation loaded: {} messages", h.messages.len());
            h
        }
        Err(e) => {
            tracing::warn!("Session not found: {} - {}", session_id, e);
            return Ok(None);
        }
    };

    let messages: Vec<Message> = history
        .messages
        .into_iter()
        .map(|m| {
            let timestamp = m
                .timestamp
                .as_ref()
                .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let content_text = m
                .content
                .iter()
                .filter_map(|c| match c {
                    ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");

            match m.role.as_str() {
                "user" | "human" => Message::User {
                    content: content_text,
                    timestamp,
                    images: vec![],
                    metadata: None,
                },
                "assistant" => Message::Assistant {
                    content: content_text,
                    timestamp,
                    model: None,
                    images: vec![],
                    metadata: None,
                },
                _ => Message::System {
                    content: content_text,
                    timestamp,
                    level: None,
                    metadata: None,
                },
            }
        })
        .collect();

    let conversation = Conversation {
        session_id: history.session_id,
        messages,
    };

    tracing::info!(
        "Conversation converted: {} messages in domain model",
        conversation.messages.len()
    );

    // OPTIMIZATION 3: Cache for next request
    cache::set_cached(&encoded_name, &session_id, conversation.clone());

    tracing::info!("Returning conversation for session: {}", session_id);
    Ok(Some(conversation))
}
