//! Conversation repository for database operations

use serde::{Deserialize, Serialize};
use surrealdb::engine::local::Db;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use chrono::{DateTime, Utc};

/// Conversation record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRecord {
    pub id: Option<Thing>,
    pub project_id: Thing,
    pub session_id: String,
    pub title: Option<String>,
    pub source_files: Vec<String>,
    pub message_count: i32,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub last_preview: String,
    pub is_favorite: bool,
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConversation {
    pub project_id: Thing,
    pub session_id: String,
    pub source_files: Vec<String>,
    pub message_count: i32,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub last_preview: String,
}

/// Summary of a conversation (for listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: Thing,
    pub session_id: String,
    pub title: Option<String>,
    pub message_count: i32,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub last_preview: String,
    pub is_favorite: bool,
    pub tags: Vec<String>,
}

/// Conversation repository
pub struct ConversationRepository;

impl ConversationRepository {
    /// Create or update a conversation (upsert by session_id)
    pub async fn upsert(
        db: &Surreal<Db>,
        conversation: CreateConversation,
    ) -> Result<ConversationRecord, String> {
        // Try to update first
        let result: Option<ConversationRecord> = db
            .query(r#"
                UPDATE conversation SET
                    source_files = $source_files,
                    message_count = $message_count,
                    start_time = $start_time,
                    end_time = $end_time,
                    last_preview = $last_preview,
                    updated_at = time::now()
                WHERE session_id = $session_id
                RETURN AFTER
            "#)
            .bind(("session_id", conversation.session_id.clone()))
            .bind(("source_files", conversation.source_files.clone()))
            .bind(("message_count", conversation.message_count))
            .bind(("start_time", conversation.start_time))
            .bind(("end_time", conversation.end_time))
            .bind(("last_preview", conversation.last_preview.clone()))
            .await
            .map_err(|e| format!("Failed to upsert conversation: {}", e))?
            .take(0)
            .map_err(|e| format!("Failed to get upsert result: {}", e))?;

        if let Some(record) = result {
            return Ok(record);
        }

        // If no update happened, insert new record
        let created: Option<ConversationRecord> = db
            .create("conversation")
            .content(ConversationRecord {
                id: None,
                project_id: conversation.project_id,
                session_id: conversation.session_id,
                title: None,
                source_files: conversation.source_files,
                message_count: conversation.message_count,
                start_time: conversation.start_time,
                end_time: conversation.end_time,
                last_preview: conversation.last_preview,
                is_favorite: false,
                tags: Vec::new(),
                notes: None,
                is_deleted: false,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
            .await
            .map_err(|e| format!("Failed to create conversation: {}", e))?;

        created.ok_or_else(|| "Failed to create conversation".to_string())
    }

    /// Get all conversations for a project (non-deleted, sorted by end_time desc)
    pub async fn find_by_project(
        db: &Surreal<Db>,
        project_id: &Thing,
    ) -> Result<Vec<ConversationSummary>, String> {
        let project_id_owned = project_id.clone();
        let conversations: Vec<ConversationSummary> = db
            .query(r#"
                SELECT id, session_id, title, message_count, start_time, end_time,
                       last_preview, is_favorite, tags
                FROM conversation
                WHERE project_id = $project_id AND is_deleted = false
                ORDER BY end_time DESC
            "#)
            .bind(("project_id", project_id_owned))
            .await
            .map_err(|e| format!("Failed to fetch conversations: {}", e))?
            .take(0)
            .map_err(|e| format!("Failed to parse conversations: {}", e))?;

        Ok(conversations)
    }

    /// Get conversation by session_id
    pub async fn find_by_session_id(
        db: &Surreal<Db>,
        session_id: &str,
    ) -> Result<Option<ConversationRecord>, String> {
        let session_id_owned = session_id.to_string();
        let mut result = db
            .query("SELECT * FROM conversation WHERE session_id = $session_id LIMIT 1")
            .bind(("session_id", session_id_owned))
            .await
            .map_err(|e| format!("Failed to query conversation: {}", e))?;

        let conversation: Option<ConversationRecord> = result
            .take(0)
            .map_err(|e| format!("Failed to get conversation: {}", e))?;

        Ok(conversation)
    }

    /// Soft delete a conversation
    pub async fn soft_delete(db: &Surreal<Db>, session_id: &str) -> Result<(), String> {
        let session_id_owned = session_id.to_string();
        db.query(r#"
            UPDATE conversation SET
                is_deleted = true,
                updated_at = time::now()
            WHERE session_id = $session_id
        "#)
        .bind(("session_id", session_id_owned))
        .await
        .map_err(|e| format!("Failed to soft delete conversation: {}", e))?;

        Ok(())
    }

    /// Hard delete a conversation (and its messages)
    pub async fn hard_delete(db: &Surreal<Db>, session_id: &str) -> Result<Vec<String>, String> {
        // Get source files before deletion
        let conversation = Self::find_by_session_id(db, session_id).await?;
        let source_files = conversation
            .map(|c| c.source_files)
            .unwrap_or_default();

        let session_id_owned = session_id.to_string();

        // Delete messages first
        db.query("DELETE FROM message WHERE conversation_id IN (SELECT id FROM conversation WHERE session_id = $session_id)")
            .bind(("session_id", session_id_owned.clone()))
            .await
            .map_err(|e| format!("Failed to delete messages: {}", e))?;

        // Delete conversation
        db.query("DELETE FROM conversation WHERE session_id = $session_id")
            .bind(("session_id", session_id_owned))
            .await
            .map_err(|e| format!("Failed to delete conversation: {}", e))?;

        Ok(source_files)
    }

    /// Update conversation metadata (title, favorite, tags, notes)
    pub async fn update_metadata(
        db: &Surreal<Db>,
        session_id: &str,
        title: Option<String>,
        is_favorite: Option<bool>,
        tags: Option<Vec<String>>,
        notes: Option<String>,
    ) -> Result<(), String> {
        let mut query = String::from("UPDATE conversation SET updated_at = time::now()");

        if title.is_some() {
            query.push_str(", title = $title");
        }
        if is_favorite.is_some() {
            query.push_str(", is_favorite = $is_favorite");
        }
        if tags.is_some() {
            query.push_str(", tags = $tags");
        }
        if notes.is_some() {
            query.push_str(", notes = $notes");
        }

        query.push_str(" WHERE session_id = $session_id");

        let session_id_owned = session_id.to_string();
        db.query(&query)
            .bind(("session_id", session_id_owned))
            .bind(("title", title))
            .bind(("is_favorite", is_favorite))
            .bind(("tags", tags))
            .bind(("notes", notes))
            .await
            .map_err(|e| format!("Failed to update conversation metadata: {}", e))?;

        Ok(())
    }

    /// Get favorite conversations
    pub async fn find_favorites(db: &Surreal<Db>) -> Result<Vec<ConversationSummary>, String> {
        let conversations: Vec<ConversationSummary> = db
            .query(r#"
                SELECT id, session_id, title, message_count, start_time, end_time,
                       last_preview, is_favorite, tags
                FROM conversation
                WHERE is_favorite = true AND is_deleted = false
                ORDER BY end_time DESC
            "#)
            .await
            .map_err(|e| format!("Failed to fetch favorites: {}", e))?
            .take(0)
            .map_err(|e| format!("Failed to parse favorites: {}", e))?;

        Ok(conversations)
    }
}
