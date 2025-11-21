//! Project repository for database operations

use serde::{Deserialize, Serialize};
use surrealdb::engine::local::Db;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use chrono::{DateTime, Utc};

/// Project record in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: Option<Thing>,
    pub name: String,
    pub path: String,
    pub encoded_name: String,
    pub ai_tool: String,
    pub session_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input for creating a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProject {
    pub name: String,
    pub path: String,
    pub encoded_name: String,
    pub ai_tool: String,
    pub session_count: i32,
}

/// Project repository
pub struct ProjectRepository;

impl ProjectRepository {
    /// Create or update a project (upsert by encoded_name)
    pub async fn upsert(db: &Surreal<Db>, project: CreateProject) -> Result<ProjectRecord, String> {
        let result: Option<ProjectRecord> = db
            .query(r#"
                UPDATE project SET
                    name = $name,
                    path = $path,
                    ai_tool = $ai_tool,
                    session_count = $session_count,
                    updated_at = time::now()
                WHERE encoded_name = $encoded_name
                RETURN AFTER
            "#)
            .bind(("name", project.name.clone()))
            .bind(("path", project.path.clone()))
            .bind(("encoded_name", project.encoded_name.clone()))
            .bind(("ai_tool", project.ai_tool.clone()))
            .bind(("session_count", project.session_count))
            .await
            .map_err(|e| format!("Failed to upsert project: {}", e))?
            .take(0)
            .map_err(|e| format!("Failed to get upsert result: {}", e))?;

        if let Some(record) = result {
            return Ok(record);
        }

        // If no update happened, insert new record
        let created: Option<ProjectRecord> = db
            .create("project")
            .content(ProjectRecord {
                id: None,
                name: project.name,
                path: project.path,
                encoded_name: project.encoded_name,
                ai_tool: project.ai_tool,
                session_count: project.session_count,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
            .await
            .map_err(|e| format!("Failed to create project: {}", e))?;

        created.ok_or_else(|| "Failed to create project".to_string())
    }

    /// Get all projects
    pub async fn find_all(db: &Surreal<Db>) -> Result<Vec<ProjectRecord>, String> {
        let projects: Vec<ProjectRecord> = db
            .select("project")
            .await
            .map_err(|e| format!("Failed to fetch projects: {}", e))?;

        Ok(projects)
    }

    /// Get project by encoded name
    pub async fn find_by_encoded_name(
        db: &Surreal<Db>,
        encoded_name: &str,
    ) -> Result<Option<ProjectRecord>, String> {
        let encoded_name_owned = encoded_name.to_string();
        let mut result = db
            .query("SELECT * FROM project WHERE encoded_name = $encoded_name LIMIT 1")
            .bind(("encoded_name", encoded_name_owned))
            .await
            .map_err(|e| format!("Failed to query project: {}", e))?;

        let project: Option<ProjectRecord> = result
            .take(0)
            .map_err(|e| format!("Failed to get project: {}", e))?;

        Ok(project)
    }

    /// Delete project by encoded name
    pub async fn delete_by_encoded_name(db: &Surreal<Db>, encoded_name: &str) -> Result<(), String> {
        let encoded_name_owned = encoded_name.to_string();
        db.query("DELETE FROM project WHERE encoded_name = $encoded_name")
            .bind(("encoded_name", encoded_name_owned))
            .await
            .map_err(|e| format!("Failed to delete project: {}", e))?;

        Ok(())
    }

    /// Update session count for a project
    pub async fn update_session_count(
        db: &Surreal<Db>,
        encoded_name: &str,
        count: i32,
    ) -> Result<(), String> {
        let encoded_name_owned = encoded_name.to_string();
        db.query(r#"
            UPDATE project SET
                session_count = $count,
                updated_at = time::now()
            WHERE encoded_name = $encoded_name
        "#)
        .bind(("encoded_name", encoded_name_owned))
        .bind(("count", count))
        .await
        .map_err(|e| format!("Failed to update session count: {}", e))?;

        Ok(())
    }
}
