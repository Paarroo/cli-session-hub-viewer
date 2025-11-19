#[cfg(any(target_arch = "wasm32", target_arch = "wasm32"))]
use reqwasm::http::Request;

use serde::de::DeserializeOwned;
use crate::domain::models::{ApiProject, Project, Session, Conversation};

// API Service for centralized HTTP requests
pub struct ApiService {
    base_url: String,
}

impl ApiService {
    pub fn new() -> Self {
        Self {
            base_url: "http://localhost:3401".to_string(),
        }
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self { base_url }
    }

    // Generic GET request
    pub async fn get<T: DeserializeOwned>(&self, endpoint: &str) -> Result<T, Box<dyn std::error::Error>> {
        let url = format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'));
        let response = Request::get(&url).send().await?;

        if !response.ok() {
            return Err(format!("HTTP {}: {}", response.status(), response.status_text()).into());
        }

        let data = response.json::<T>().await?;
        Ok(data)
    }

    // Generic POST request
    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        endpoint: &str,
        body: &B
    ) -> Result<T, Box<dyn std::error::Error>> {
        let url = format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'));
        let response = Request::post(&url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(body)?)
            .send()
            .await?;

        if !response.ok() {
            return Err(format!("HTTP {}: {}", response.status(), response.status_text()).into());
        }

        let data = response.json::<T>().await?;
        Ok(data)
    }

    // Generic DELETE request
    pub async fn delete(&self, endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'));
        let response = Request::delete(&url).send().await?;

        if !response.ok() {
            return Err(format!("HTTP {}: {}", response.status(), response.status_text()).into());
        }

        Ok(())
    }

    // Specific API methods
    pub async fn get_projects(&self) -> Result<Vec<ApiProject>, Box<dyn std::error::Error>> {
        self.get("/api/projects").await
    }

    pub async fn get_sessions(&self, project_id: &str) -> Result<Vec<Session>, Box<dyn std::error::Error>> {
        self.get(&format!("/api/projects/{}/sessions", project_id)).await
    }

    pub async fn get_conversation(&self, project_name: &str, session_id: &str) -> Result<Conversation, Box<dyn std::error::Error>> {
        self.get(&format!("/api/conversation/{}/{}", project_name, session_id)).await
    }

    pub async fn delete_project(&self, project_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.delete(&format!("/api/projects/{}", project_id)).await
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.delete(&format!("/api/sessions/{}", session_id)).await
    }
}

impl Default for ApiService {
    fn default() -> Self {
        Self::new()
    }
}