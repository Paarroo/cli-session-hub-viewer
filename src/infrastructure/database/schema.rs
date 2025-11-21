//! Database schema definitions and migrations
//!
//! Defines tables for: project, conversation, message
//! Uses SurrealQL for schema definitions

use surrealdb::engine::local::Db;
use surrealdb::Surreal;

/// Run all database migrations
pub async fn run_migrations(db: &Surreal<Db>) -> Result<(), String> {
    tracing::info!("Running database migrations...");

    // Create project table
    create_project_table(db).await?;

    // Create conversation table
    create_conversation_table(db).await?;

    // Create message table
    create_message_table(db).await?;

    tracing::info!("Database migrations completed");
    Ok(())
}

async fn create_project_table(db: &Surreal<Db>) -> Result<(), String> {
    db.query(r#"
        DEFINE TABLE IF NOT EXISTS project SCHEMAFULL;

        DEFINE FIELD IF NOT EXISTS name ON project TYPE string;
        DEFINE FIELD IF NOT EXISTS path ON project TYPE string;
        DEFINE FIELD IF NOT EXISTS encoded_name ON project TYPE string;
        DEFINE FIELD IF NOT EXISTS ai_tool ON project TYPE string;
        DEFINE FIELD IF NOT EXISTS session_count ON project TYPE int DEFAULT 0;
        DEFINE FIELD IF NOT EXISTS created_at ON project TYPE datetime DEFAULT time::now();
        DEFINE FIELD IF NOT EXISTS updated_at ON project TYPE datetime DEFAULT time::now();

        DEFINE INDEX IF NOT EXISTS idx_project_encoded ON project FIELDS encoded_name UNIQUE;
        DEFINE INDEX IF NOT EXISTS idx_project_path ON project FIELDS path;
    "#)
    .await
    .map_err(|e| format!("Failed to create project table: {}", e))?;

    Ok(())
}

async fn create_conversation_table(db: &Surreal<Db>) -> Result<(), String> {
    db.query(r#"
        DEFINE TABLE IF NOT EXISTS conversation SCHEMAFULL;

        DEFINE FIELD IF NOT EXISTS project_id ON conversation TYPE record<project>;
        DEFINE FIELD IF NOT EXISTS session_id ON conversation TYPE string;
        DEFINE FIELD IF NOT EXISTS title ON conversation TYPE option<string>;
        DEFINE FIELD IF NOT EXISTS source_files ON conversation TYPE array DEFAULT [];
        DEFINE FIELD IF NOT EXISTS message_count ON conversation TYPE int DEFAULT 0;
        DEFINE FIELD IF NOT EXISTS start_time ON conversation TYPE datetime;
        DEFINE FIELD IF NOT EXISTS end_time ON conversation TYPE datetime;
        DEFINE FIELD IF NOT EXISTS last_preview ON conversation TYPE string DEFAULT '';
        DEFINE FIELD IF NOT EXISTS is_favorite ON conversation TYPE bool DEFAULT false;
        DEFINE FIELD IF NOT EXISTS tags ON conversation TYPE array DEFAULT [];
        DEFINE FIELD IF NOT EXISTS notes ON conversation TYPE option<string>;
        DEFINE FIELD IF NOT EXISTS is_deleted ON conversation TYPE bool DEFAULT false;
        DEFINE FIELD IF NOT EXISTS created_at ON conversation TYPE datetime DEFAULT time::now();
        DEFINE FIELD IF NOT EXISTS updated_at ON conversation TYPE datetime DEFAULT time::now();

        DEFINE INDEX IF NOT EXISTS idx_conv_session ON conversation FIELDS session_id UNIQUE;
        DEFINE INDEX IF NOT EXISTS idx_conv_project ON conversation FIELDS project_id;
        DEFINE INDEX IF NOT EXISTS idx_conv_favorite ON conversation FIELDS is_favorite;
        DEFINE INDEX IF NOT EXISTS idx_conv_deleted ON conversation FIELDS is_deleted;
    "#)
    .await
    .map_err(|e| format!("Failed to create conversation table: {}", e))?;

    Ok(())
}

async fn create_message_table(db: &Surreal<Db>) -> Result<(), String> {
    db.query(r#"
        DEFINE TABLE IF NOT EXISTS message SCHEMAFULL;

        DEFINE FIELD IF NOT EXISTS conversation_id ON message TYPE record<conversation>;
        DEFINE FIELD IF NOT EXISTS message_id ON message TYPE option<string>;
        DEFINE FIELD IF NOT EXISTS role ON message TYPE string;
        DEFINE FIELD IF NOT EXISTS content ON message TYPE array DEFAULT [];
        DEFINE FIELD IF NOT EXISTS timestamp ON message TYPE datetime;
        DEFINE FIELD IF NOT EXISTS uuid ON message TYPE option<string>;
        DEFINE FIELD IF NOT EXISTS parent_uuid ON message TYPE option<string>;
        DEFINE FIELD IF NOT EXISTS is_sidechain ON message TYPE bool DEFAULT false;
        DEFINE FIELD IF NOT EXISTS created_at ON message TYPE datetime DEFAULT time::now();

        DEFINE INDEX IF NOT EXISTS idx_msg_conversation ON message FIELDS conversation_id;
        DEFINE INDEX IF NOT EXISTS idx_msg_message_id ON message FIELDS message_id;
        DEFINE INDEX IF NOT EXISTS idx_msg_timestamp ON message FIELDS timestamp;
        DEFINE INDEX IF NOT EXISTS idx_msg_role ON message FIELDS role;
    "#)
    .await
    .map_err(|e| format!("Failed to create message table: {}", e))?;

    Ok(())
}
