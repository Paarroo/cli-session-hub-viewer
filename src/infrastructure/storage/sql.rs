/// SQL constants for database operations (DRY principle)
pub mod sql {

    // Table creation
    pub const CREATE_PROJECTS_TABLE: &str = r#"
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            ai_tool TEXT NOT NULL,
            session_count INTEGER DEFAULT 0,
            last_modified TIMESTAMPTZ NOT NULL,
            created_at TIMESTAMPTZ NOT NULL
        )
    "#;

    pub const CREATE_SESSIONS_TABLE: &str = r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            project_name TEXT NOT NULL,
            ai_tool TEXT NOT NULL,
            message_count INTEGER DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL,
            updated_at TIMESTAMPTZ NOT NULL,
            summary TEXT,
            status TEXT NOT NULL,
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        )
    "#;

    pub const CREATE_MESSAGES_TABLE: &str = r#"
        CREATE TABLE IF NOT EXISTS messages (
            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
            session_id TEXT NOT NULL,
            type TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp TIMESTAMPTZ NOT NULL,
            metadata JSONB,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        )
    "#;

    // Indexes
    pub const CREATE_INDEXES: &str = r#"
        CREATE INDEX IF NOT EXISTS idx_sessions_project_id ON sessions(project_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_updated_at ON sessions(updated_at);
        CREATE INDEX IF NOT EXISTS idx_messages_session_id ON messages(session_id);
        CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
    "#;

    // Project queries
    pub const INSERT_PROJECT: &str = r#"
        INSERT INTO projects (id, name, path, ai_tool, session_count, last_modified, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            path = EXCLUDED.path,
            ai_tool = EXCLUDED.ai_tool,
            session_count = EXCLUDED.session_count,
            last_modified = EXCLUDED.last_modified
    "#;

    pub const SELECT_ALL_PROJECTS: &str = r#"
        SELECT id, name, path, ai_tool, session_count, last_modified, created_at
        FROM projects ORDER BY last_modified DESC
    "#;

    pub const SELECT_PROJECT_BY_ID: &str = r#"
        SELECT id, name, path, ai_tool, session_count, last_modified, created_at
        FROM projects WHERE id = $1
    "#;

    pub const SELECT_PROJECT_BY_NAME: &str = r#"
        SELECT id, name, path, ai_tool, session_count, last_modified, created_at
        FROM projects WHERE name = $1
    "#;

    pub const UPDATE_PROJECT_SESSION_COUNT: &str = r#"
        UPDATE projects SET session_count = $1, last_modified = $2 WHERE id = $3
    "#;

    pub const DELETE_PROJECT: &str = "DELETE FROM projects WHERE id = $1";

    // Session queries
    pub const INSERT_SESSION: &str = r#"
        INSERT INTO sessions
        (id, project_id, project_name, ai_tool, message_count, created_at, updated_at, summary, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        ON CONFLICT (id) DO UPDATE SET
            project_name = EXCLUDED.project_name,
            ai_tool = EXCLUDED.ai_tool,
            message_count = EXCLUDED.message_count,
            updated_at = EXCLUDED.updated_at,
            summary = EXCLUDED.summary,
            status = EXCLUDED.status
    "#;

    pub const SELECT_SESSIONS_BY_PROJECT: &str = r#"
        SELECT id, project_id, project_name, ai_tool, message_count, created_at, updated_at, summary, status
        FROM sessions WHERE project_id = $1 ORDER BY updated_at DESC
    "#;

    pub const SELECT_SESSION_BY_ID: &str = r#"
        SELECT id, project_id, project_name, ai_tool, message_count, created_at, updated_at, summary, status
        FROM sessions WHERE id = $1
    "#;

    pub const UPDATE_SESSION_STATUS: &str = r#"
        UPDATE sessions SET status = $1, updated_at = $2 WHERE id = $3
    "#;

    pub const DELETE_SESSION: &str = "DELETE FROM sessions WHERE id = $1";

    // Message queries
    pub const INSERT_MESSAGE: &str = r#"
        INSERT INTO messages (session_id, type, content, timestamp, metadata)
        VALUES ($1, $2, $3, $4, $5)
    "#;

    pub const SELECT_MESSAGES_BY_SESSION: &str = r#"
        SELECT metadata FROM messages WHERE session_id = $1 ORDER BY timestamp ASC
    "#;
}