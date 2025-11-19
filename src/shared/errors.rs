use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("File operation error: {0}")]
    FileError(String),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    // PostgreSQL support removed - using filesystem-based .jsonl parsing
    // #[error("PostgreSQL error: {0}")]
    // PostgresError(#[from] tokio_postgres::Error),
}

pub type Result<T> = std::result::Result<T, AppError>;
