use axum::{
    extract::Query,
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::models::AiTool;
use crate::history::{list_projects, ProjectInfo};

#[derive(Debug, Deserialize)]
pub struct ProjectsQuery {
    #[serde(default)]
    pub search: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub name: String,
    pub path: String,
    pub session_count: usize,
    pub ai_tool: AiTool,
    pub encoded_name: String,
    pub last_updated: DateTime<Utc>,
}

impl From<ProjectInfo> for ProjectResponse {
    fn from(info: ProjectInfo) -> Self {
        Self {
            name: info.name,
            path: info.path,
            session_count: info.session_count,
            ai_tool: info.ai_tool,
            encoded_name: info.encoded_name,
            last_updated: info.last_updated,
        }
    }
}

/// GET /api/projects
/// List all projects from filesystem
pub async fn list_projects_handler(
    Query(params): Query<ProjectsQuery>,
) -> Result<Json<Vec<ProjectResponse>>, StatusCode> {
    let mut projects = list_projects()
        .map_err(|e| {
            tracing::error!("Failed to list projects: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Filter by search query if provided
    if !params.search.is_empty() {
        let search_lower = params.search.to_lowercase();
        projects.retain(|p| {
            p.name.to_lowercase().contains(&search_lower)
                || p.path.to_lowercase().contains(&search_lower)
        });
    }

    // Sort by last_updated descending (most recent first)
    projects.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));

    let response: Vec<ProjectResponse> = projects
        .into_iter()
        .map(ProjectResponse::from)
        .collect();

    Ok(Json(response))
}
