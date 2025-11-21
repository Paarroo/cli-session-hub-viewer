//! SurrealDB database module for conversation history management
//!
//! This module provides:
//! - Embedded SurrealDB connection (~/.claude-viewer/surreal/)
//! - Schema definitions for projects, conversations, and messages
//! - Repository layer for CRUD operations
//! - Import/sync utilities from Claude's JSONL files

pub mod connection;
pub mod schema;
pub mod repositories;
pub mod importer;

pub use connection::{Database, init_database, get_database, try_get_database};
pub use schema::run_migrations;
pub use repositories::{
    ProjectRepository,
    ConversationRepository,
    MessageRepository,
};
pub use importer::{import_all_history, sync_project, ImportStats};
