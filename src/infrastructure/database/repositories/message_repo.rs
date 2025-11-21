//! Message repository for database operations

use serde::{Deserialize, Serialize};
use surrealdb::engine::local::Db;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use chrono::{DateTime, Utc};

/// Message record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: Option<Thing>,
    pub conversation_id: Thing,
    pub message_id: Option<String>,
    pub role: String,
    pub content: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub uuid: Option<String>,
    pub parent_uuid: Option<String>,
    pub is_sidechain: bool,
    pub created_at: DateTime<Utc>,
}

/// Input for creating a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessage {
    pub conversation_id: Thing,
    pub message_id: Option<String>,
    pub role: String,
    pub content: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub uuid: Option<String>,
    pub parent_uuid: Option<String>,
    pub is_sidechain: bool,
}

/// Message repository
pub struct MessageRepository;

impl MessageRepository {
    /// Create a message
    pub async fn create(db: &Surreal<Db>, message: CreateMessage) -> Result<MessageRecord, String> {
        let created: Option<MessageRecord> = db
            .create("message")
            .content(MessageRecord {
                id: None,
                conversation_id: message.conversation_id,
                message_id: message.message_id,
                role: message.role,
                content: message.content,
                timestamp: message.timestamp,
                uuid: message.uuid,
                parent_uuid: message.parent_uuid,
                is_sidechain: message.is_sidechain,
                created_at: Utc::now(),
            })
            .await
            .map_err(|e| format!("Failed to create message: {}", e))?;

        created.ok_or_else(|| "Failed to create message".to_string())
    }

    /// Create multiple messages in batch
    pub async fn create_batch(
        db: &Surreal<Db>,
        messages: Vec<CreateMessage>,
    ) -> Result<usize, String> {
        if messages.is_empty() {
            return Ok(0);
        }

        let records: Vec<MessageRecord> = messages
            .into_iter()
            .map(|m| MessageRecord {
                id: None,
                conversation_id: m.conversation_id,
                message_id: m.message_id,
                role: m.role,
                content: m.content,
                timestamp: m.timestamp,
                uuid: m.uuid,
                parent_uuid: m.parent_uuid,
                is_sidechain: m.is_sidechain,
                created_at: Utc::now(),
            })
            .collect();

        let count = records.len();

        // Insert messages one by one (SurrealDB doesn't have bulk insert)
        for record in records {
            let _: Option<MessageRecord> = db
                .create("message")
                .content(record)
                .await
                .map_err(|e| format!("Failed to create message: {}", e))?;
        }

        Ok(count)
    }

    /// Get all messages for a conversation (sorted by timestamp)
    pub async fn find_by_conversation(
        db: &Surreal<Db>,
        conversation_id: &Thing,
    ) -> Result<Vec<MessageRecord>, String> {
        let conversation_id_owned = conversation_id.clone();
        let messages: Vec<MessageRecord> = db
            .query(r#"
                SELECT *
                FROM message
                WHERE conversation_id = $conversation_id
                ORDER BY timestamp ASC
            "#)
            .bind(("conversation_id", conversation_id_owned))
            .await
            .map_err(|e| format!("Failed to fetch messages: {}", e))?
            .take(0)
            .map_err(|e| format!("Failed to parse messages: {}", e))?;

        Ok(messages)
    }

    /// Get messages by session_id (joins with conversation)
    pub async fn find_by_session_id(
        db: &Surreal<Db>,
        session_id: &str,
    ) -> Result<Vec<MessageRecord>, String> {
        let session_id_owned = session_id.to_string();
        let messages: Vec<MessageRecord> = db
            .query(r#"
                SELECT message.*
                FROM message
                WHERE conversation_id IN (
                    SELECT id FROM conversation WHERE session_id = $session_id
                )
                ORDER BY message.timestamp ASC
            "#)
            .bind(("session_id", session_id_owned))
            .await
            .map_err(|e| format!("Failed to fetch messages by session: {}", e))?
            .take(0)
            .map_err(|e| format!("Failed to parse messages: {}", e))?;

        Ok(messages)
    }

    /// Delete all messages for a conversation
    pub async fn delete_by_conversation(
        db: &Surreal<Db>,
        conversation_id: &Thing,
    ) -> Result<(), String> {
        let conversation_id_owned = conversation_id.clone();
        db.query("DELETE FROM message WHERE conversation_id = $conversation_id")
            .bind(("conversation_id", conversation_id_owned))
            .await
            .map_err(|e| format!("Failed to delete messages: {}", e))?;

        Ok(())
    }

    /// Check if a message with given message_id exists for a conversation
    pub async fn exists_by_message_id(
        db: &Surreal<Db>,
        conversation_id: &Thing,
        message_id: &str,
    ) -> Result<bool, String> {
        let conversation_id_owned = conversation_id.clone();
        let message_id_owned = message_id.to_string();
        let result: Option<MessageRecord> = db
            .query(r#"
                SELECT * FROM message
                WHERE conversation_id = $conversation_id AND message_id = $message_id
                LIMIT 1
            "#)
            .bind(("conversation_id", conversation_id_owned))
            .bind(("message_id", message_id_owned))
            .await
            .map_err(|e| format!("Failed to check message existence: {}", e))?
            .take(0)
            .map_err(|e| format!("Failed to parse result: {}", e))?;

        Ok(result.is_some())
    }

    /// Count messages for a conversation
    pub async fn count_by_conversation(
        db: &Surreal<Db>,
        conversation_id: &Thing,
    ) -> Result<i32, String> {
        let conversation_id_owned = conversation_id.clone();
        let result: Option<i32> = db
            .query("SELECT count() FROM message WHERE conversation_id = $conversation_id GROUP ALL")
            .bind(("conversation_id", conversation_id_owned))
            .await
            .map_err(|e| format!("Failed to count messages: {}", e))?
            .take("count")
            .map_err(|e| format!("Failed to get count: {}", e))?;

        Ok(result.unwrap_or(0))
    }

    /// Search messages by content (full-text search)
    pub async fn search(
        db: &Surreal<Db>,
        query: &str,
        limit: i32,
    ) -> Result<Vec<MessageRecord>, String> {
        let query_owned = query.to_string();
        let messages: Vec<MessageRecord> = db
            .query(r#"
                SELECT *
                FROM message
                WHERE string::contains(string::lowercase(content), string::lowercase($query))
                ORDER BY timestamp DESC
                LIMIT $limit
            "#)
            .bind(("query", query_owned))
            .bind(("limit", limit))
            .await
            .map_err(|e| format!("Failed to search messages: {}", e))?
            .take(0)
            .map_err(|e| format!("Failed to parse search results: {}", e))?;

        Ok(messages)
    }
}
