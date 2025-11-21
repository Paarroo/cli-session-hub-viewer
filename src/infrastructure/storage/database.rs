use chrono::{DateTime, Utc};
use bb8_postgres::bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use tokio_postgres::NoTls;
use crate::domain::models::{AiTool, Project, Session, SessionStatus};

// SQL Constants
const CREATE_PROJECTS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS projects (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        path TEXT NOT NULL UNIQUE,
        ai_tool TEXT NOT NULL,
        session_count INTEGER DEFAULT 0,
        last_modified TEXT NOT NULL,
        created_at TEXT NOT NULL
    )
"#;

const CREATE_SESSIONS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL,
        project_name TEXT NOT NULL,
        ai_tool TEXT NOT NULL,
        message_count INTEGER DEFAULT 0,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL,
        summary TEXT,
        status TEXT NOT NULL,
        FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
    )
"#;

const CREATE_MESSAGES_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS messages (
        id TEXT PRIMARY KEY,
        session_id TEXT NOT NULL,
        type TEXT NOT NULL,
        content TEXT NOT NULL,
        timestamp TEXT NOT NULL,
        metadata TEXT,
        FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
    )
"#;

const INSERT_PROJECT: &str = r#"
    INSERT INTO projects (id, name, path, ai_tool, session_count, last_modified, created_at)
    VALUES ($1, $2, $3, $4, $5, $6, $7)
    ON CONFLICT (id) DO UPDATE SET
        name = EXCLUDED.name,
        path = EXCLUDED.path,
        ai_tool = EXCLUDED.ai_tool,
        session_count = EXCLUDED.session_count,
        last_modified = EXCLUDED.last_modified
"#;

const SELECT_ALL_PROJECTS: &str = r#"
    SELECT id, name, path, ai_tool, session_count, last_modified, created_at
    FROM projects ORDER BY last_modified DESC
"#;

const SELECT_PROJECT_BY_ID: &str = r#"
    SELECT id, name, path, ai_tool, session_count, last_modified, created_at
    FROM projects WHERE id = $1
"#;

const DELETE_PROJECT: &str = "DELETE FROM projects WHERE id = $1";

const SELECT_SESSION_BY_ID: &str = r#"
    SELECT id, project_id, project_name, ai_tool, message_count, created_at, updated_at, summary, status
    FROM sessions WHERE id = $1
"#;

const UPDATE_SESSION_STATUS: &str = r#"
    UPDATE sessions SET status = $1, updated_at = $2 WHERE id = $3
"#;

const DELETE_SESSION: &str = "DELETE FROM sessions WHERE id = $1";

const INSERT_SESSION: &str = r#"
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

const SELECT_SESSIONS_BY_PROJECT: &str = r#"
    SELECT id, project_id, project_name, ai_tool, message_count, created_at, updated_at, summary, status
    FROM sessions WHERE project_id = $1 ORDER BY updated_at DESC
"#;

const INSERT_MESSAGE: &str = r#"
    INSERT INTO messages (id, session_id, type, content, timestamp, metadata)
    VALUES ($1, $2, $3, $4, $5, $6)
"#;

const SELECT_MESSAGES_BY_SESSION: &str = r#"
    SELECT id, session_id, type, content, timestamp, metadata FROM messages WHERE session_id = $1 ORDER BY timestamp ASC
"#;

pub type DbPool = Pool<PostgresConnectionManager<NoTls>>;

/// Initialize database connection pool
pub async fn init_db_pool(database_url: &str) -> Result<DbPool, Box<dyn std::error::Error>> {
    let manager = PostgresConnectionManager::new_from_stringlike(database_url, NoTls)?;
    let pool = Pool::builder().build(manager).await?;

    // Initialize database tables
    {
        let conn = pool.get().await?;
        conn.execute(CREATE_PROJECTS_TABLE, &[]).await?;
        conn.execute(CREATE_SESSIONS_TABLE, &[]).await?;
        conn.execute(CREATE_MESSAGES_TABLE, &[]).await?;
    }

    Ok(pool)
}



/// Project repository
pub struct ProjectRepository {
    pool: DbPool,
}

impl ProjectRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create_project(&self, project: &Project) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        conn.execute(
            INSERT_PROJECT,
            &[
                &project.id,
                &project.name,
                &project.path,
                &serde_json::to_string(&project.ai_tool).unwrap(),
                &(project.session_count as i32),
                &project.last_modified.to_rfc3339(),
                &project.created_at.to_rfc3339(),
            ],
        ).await?;
        Ok(())
    }

    pub async fn get_all_projects(&self) -> Result<Vec<Project>, Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let rows = conn.query(SELECT_ALL_PROJECTS, &[]).await?;

        let mut projects = Vec::new();
        for row in rows {
            let ai_tool_str: String = row.get(3);
            let ai_tool: AiTool = serde_json::from_str(&ai_tool_str).unwrap_or(AiTool::ClaudeCode);

            let session_count_i32: i32 = row.get(4);
            let last_modified_str: String = row.get(5);
            let created_at_str: String = row.get(6);

            projects.push(Project {
                encoded_name: String::new(),
                id: row.get(0),
                name: row.get(1),
                path: row.get(2),
                ai_tool,
                session_count: session_count_i32,
                last_modified: DateTime::<Utc>::from(DateTime::parse_from_rfc3339(&last_modified_str).unwrap_or_else(|_| Utc::now().into())),
                created_at: DateTime::<Utc>::from(DateTime::parse_from_rfc3339(&created_at_str).unwrap_or_else(|_| Utc::now().into())),
            });
        }

        Ok(projects)
    }

    pub async fn get_project_by_id(
        &self,
        id: &str,
    ) -> Result<Option<Project>, Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let rows = conn.query(SELECT_PROJECT_BY_ID, &[&id]).await?;

        if let Some(row) = rows.into_iter().next() {
            let ai_tool_str: String = row.get(3);
            let ai_tool: AiTool = serde_json::from_str(&ai_tool_str).unwrap_or(AiTool::ClaudeCode);

            let session_count_i32: i32 = row.get(4);
            let last_modified_str: String = row.get(5);
            let created_at_str: String = row.get(6);

            let project = Project {
                encoded_name: String::new(),
                id: row.get(0),
                name: row.get(1),
                path: row.get(2),
                ai_tool,
                session_count: session_count_i32,
                last_modified: DateTime::<Utc>::from(DateTime::parse_from_rfc3339(&last_modified_str).unwrap_or_else(|_| Utc::now().into())),
                created_at: DateTime::<Utc>::from(DateTime::parse_from_rfc3339(&created_at_str).unwrap_or_else(|_| Utc::now().into())),
            };

            return Ok(Some(project));
        }

        Ok(None)
    }

    pub async fn get_session_by_id(
        &self,
        id: &str,
    ) -> Result<Option<Session>, Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let rows = conn.query(SELECT_SESSION_BY_ID, &[&id]).await?;

        if let Some(row) = rows.into_iter().next() {
            let ai_tool_str: String = row.get(3);
            let ai_tool: AiTool = serde_json::from_str(&ai_tool_str).unwrap_or(AiTool::ClaudeCode);

            let status_str: String = row.get(8);
            let status: SessionStatus =
                serde_json::from_str(&status_str).unwrap_or(SessionStatus::Completed);

            Ok(Some(Session {
                last_message_preview: String::new(),
                last_time: String::new(),
                id: row.get(0),
                project_id: row.get(1),
                project_name: row.get(2),
                ai_tool,
                message_count: row.get::<_, i32>(4) as usize,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5))?.with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6))?.with_timezone(&Utc),
                summary: row.get(7),
                status,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_session_status(
        &self,
        session_id: &str,
        status: SessionStatus,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        conn.execute(
            UPDATE_SESSION_STATUS,
            &[
                &serde_json::to_string(&status).unwrap(),
                &Utc::now().to_rfc3339(),
                &session_id
            ],
        ).await?;
        Ok(())
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        conn.execute(DELETE_SESSION, &[&session_id]).await?;
        Ok(())
    }

    pub async fn delete_project(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        conn.execute(DELETE_PROJECT, &[&id]).await?;
        Ok(())
    }
}

/// Session repository
pub struct SessionRepository {
    pool: DbPool,
}

impl SessionRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create_session(&self, session: &Session) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        conn.execute(
            INSERT_SESSION,
            &[
                &session.id,
                &session.project_id,
                &session.project_name,
                &serde_json::to_string(&session.ai_tool).unwrap(),
                &(session.message_count as i32),
                &session.created_at.to_rfc3339(),
                &session.updated_at.to_rfc3339(),
                &session.summary,
                &serde_json::to_string(&session.status).unwrap(),
            ],
        ).await?;
        Ok(())
    }

    pub async fn get_sessions_by_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<Session>, Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let rows = conn.query(SELECT_SESSIONS_BY_PROJECT, &[&project_id]).await?;

        let mut sessions = Vec::new();
        for row in rows {
            let ai_tool_str: String = row.get(3);
            let ai_tool: AiTool = serde_json::from_str(&ai_tool_str).unwrap_or(AiTool::ClaudeCode);

            let message_count_i32: i32 = row.get(4);
            let created_at_str: String = row.get(5);
            let updated_at_str: String = row.get(6);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc);
            let summary: String = row.get(7);
            let status_str: String = row.get(8);
            let status: SessionStatus =
                serde_json::from_str(&status_str).unwrap_or(SessionStatus::Completed);

            sessions.push(Session {
                last_message_preview: String::new(),
                last_time: String::new(),
                id: row.get(0),
                project_id: row.get(1),
                project_name: row.get(2),
                ai_tool,
                message_count: message_count_i32 as usize,
                created_at: DateTime::<Utc>::from(DateTime::parse_from_rfc3339(&created_at_str).unwrap_or_else(|_| Utc::now().into())),
                updated_at,
                summary,
                status,
            });
        }

        Ok(sessions)
    }

    pub async fn get_session_by_id(
        &self,
        id: &str,
    ) -> Result<Option<Session>, Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let rows = conn.query(SELECT_SESSION_BY_ID, &[&id]).await?;

        if let Some(row) = rows.into_iter().next() {
            let ai_tool_str: String = row.get(3);
            let ai_tool: AiTool = serde_json::from_str(&ai_tool_str).unwrap_or(AiTool::ClaudeCode);

            let message_count_i32: i32 = row.get(4);
            let created_at_str: String = row.get(5);
            let updated_at_str: String = row.get(6);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc);
            let summary: String = row.get(7);
            let status_str: String = row.get(8);
            let status: SessionStatus =
                serde_json::from_str(&status_str).unwrap_or(SessionStatus::Completed);

            Ok(Some(Session {
                last_message_preview: String::new(),
                last_time: String::new(),
                id: row.get(0),
                project_id: row.get(1),
                project_name: row.get(2),
                ai_tool,
                message_count: message_count_i32 as usize,
                created_at: DateTime::<Utc>::from(DateTime::parse_from_rfc3339(&created_at_str).unwrap_or_else(|_| Utc::now().into())),
                updated_at,
                summary,
                status,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn update_session_status(
        &self,
        session_id: &str,
        status: SessionStatus,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        conn.execute(
            UPDATE_SESSION_STATUS,
            &[
                &serde_json::to_string(&status).unwrap(),
                &Utc::now().to_rfc3339(),
                &session_id
            ],
        ).await?;
        Ok(())
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        conn.execute(DELETE_SESSION, &[&session_id]).await?;
        Ok(())
    }
}

/// Message repository
pub struct MessageRepository {
    pool: DbPool,
}

impl MessageRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn store_messages(
        &self,
        session_id: &str,
        messages: &[crate::domain::models::Message],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        for message in messages {
            let message_type = match message {
                crate::domain::models::Message::User { .. } => "user",
                crate::domain::models::Message::Assistant { .. } => "assistant",
                crate::domain::models::Message::Tool { .. } => "tool",
                crate::domain::models::Message::System { .. } => "system",
            };

            let content = match message {
                crate::domain::models::Message::User { content, .. } => content.clone(),
                crate::domain::models::Message::Assistant { content, .. } => content.clone(),
                crate::domain::models::Message::Tool { output, .. } => output.clone().unwrap_or_default(),
                crate::domain::models::Message::System { content, .. } => content.clone(),
            };

            let timestamp = match message {
                crate::domain::models::Message::User { timestamp, .. } => *timestamp,
                crate::domain::models::Message::Assistant { timestamp, .. } => *timestamp,
                crate::domain::models::Message::Tool { timestamp, .. } => *timestamp,
                crate::domain::models::Message::System { timestamp, .. } => *timestamp,
            };

            let metadata = serde_json::to_value(message)?;

            conn.execute(
                INSERT_MESSAGE,
                &[
                    &session_id,
                    &message_type,
                    &content,
                    &timestamp.to_rfc3339(),
                    &serde_json::to_string(&metadata)?,
                ],
            ).await?;
        }

        Ok(())
    }

    pub async fn get_messages_by_session(
        &self,
        session_id: &str,
    ) -> Result<Vec<crate::domain::models::Message>, Box<dyn std::error::Error>> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let rows = conn.query(SELECT_MESSAGES_BY_SESSION, &[&session_id]).await?;

        let mut messages = Vec::new();
        for row in rows {
            let metadata_str: String = row.get(0);
            let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;
            let message: crate::domain::models::Message = serde_json::from_value(metadata)?;
            messages.push(message);
        }

        Ok(messages)
    }
}
