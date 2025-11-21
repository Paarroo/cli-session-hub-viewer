//! SurrealDB connection management
//!
//! Provides embedded database connection stored in ~/.claude-viewer/surreal/

use surrealdb::engine::local::{Db, RocksDb};
use surrealdb::Surreal;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;

use super::schema::run_migrations;

/// Database connection wrapper
pub type Database = Arc<Surreal<Db>>;

/// Global database instance (singleton)
static DB: OnceCell<Database> = OnceCell::const_new();

/// Get the database directory path (~/.claude-viewer/surreal/)
fn get_db_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME")
        .map_err(|_| "HOME environment variable not set".to_string())?;

    let db_path = PathBuf::from(home)
        .join(".claude-viewer")
        .join("surreal");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&db_path)
        .map_err(|e| format!("Failed to create database directory: {}", e))?;

    Ok(db_path)
}

/// Initialize the database connection
/// This should be called once at application startup
pub async fn init_database() -> Result<Database, String> {
    // Return existing connection if already initialized
    if let Some(db) = DB.get() {
        return Ok(db.clone());
    }

    let db_path = get_db_path()?;

    tracing::info!("Initializing SurrealDB at {:?}", db_path);

    // Connect to embedded RocksDB
    let db = Surreal::new::<RocksDb>(db_path)
        .await
        .map_err(|e| format!("Failed to connect to SurrealDB: {}", e))?;

    // Select namespace and database
    db.use_ns("claude_viewer")
        .use_db("main")
        .await
        .map_err(|e| format!("Failed to select namespace/database: {}", e))?;

    // Run migrations
    run_migrations(&db).await?;

    let db = Arc::new(db);

    // Store in global singleton
    DB.set(db.clone())
        .map_err(|_| "Database already initialized".to_string())?;

    tracing::info!("SurrealDB initialized successfully");

    Ok(db)
}

/// Get the database connection (panics if not initialized)
pub fn get_database() -> Database {
    DB.get()
        .expect("Database not initialized. Call init_database() first.")
        .clone()
}

/// Try to get the database connection (returns None if not initialized)
pub fn try_get_database() -> Option<Database> {
    DB.get().cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_db_path() {
        let result = get_db_path();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains(".claude-viewer"));
    }
}
