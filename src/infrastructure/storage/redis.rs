use bb8_redis::bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::AsyncCommands;
use chrono::{DateTime, Utc};
use crate::domain::models::{AiTool, Project, SearchQuery, SearchResult, Session};

pub type RedisPool = Pool<RedisConnectionManager>;

/// Initialize Redis connection pool
pub async fn init_redis_pool(redis_url: &str) -> Result<RedisPool, Box<dyn std::error::Error>> {
    let manager = RedisConnectionManager::new(redis_url)?;
    let pool = Pool::builder().build(manager).await?;
    Ok(pool)
}

/// Cache keys
mod keys {
    use crate::domain::models::AiTool;

    pub fn project_key(project_id: &str) -> String {
        format!("project:{}", project_id)
    }

    pub fn session_key(session_id: &str) -> String {
        format!("session:{}", session_id)
    }

    pub fn project_sessions_key(project_id: &str) -> String {
        format!("project:{}:sessions", project_id)
    }

    pub fn ai_tool_projects_key(ai_tool: &AiTool) -> String {
        format!("ai_tool:{:?}:projects", ai_tool)
    }

    pub fn search_index_key() -> &'static str {
        "search:index"
    }

    pub fn session_messages_key(session_id: &str) -> String {
        format!("session:{}:messages", session_id)
    }
}

/// Project cache operations
pub struct ProjectCache {
    pool: RedisPool,
}

impl ProjectCache {
    pub fn new(pool: RedisPool) -> Self {
        Self { pool }
    }

    pub async fn store_project(&self, project: &Project) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let key = keys::project_key(&project.id);
        let data = serde_json::to_string(project)?;

        // Store project data
        let _: () = conn.set_ex(&key, &data, 3600).await?; // 1 hour TTL

        // Add to AI tool index
        let ai_tool_key = keys::ai_tool_projects_key(&project.ai_tool);
        let _: () = conn.sadd(&ai_tool_key, &project.id).await?;

        Ok(())
    }

    pub async fn get_project(
        &self,
        project_id: &str,
    ) -> Result<Option<Project>, Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let key = keys::project_key(project_id);

        let data: Option<String> = conn.get(&key).await?;
        match data {
            Some(json) => {
                let project: Project = serde_json::from_str(&json)?;
                Ok(Some(project))
            }
            None => Ok(None),
        }
    }

    pub async fn get_projects_by_ai_tool(
        &self,
        ai_tool: &AiTool,
    ) -> Result<Vec<Project>, Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let ai_tool_key = keys::ai_tool_projects_key(ai_tool);

        let project_ids: Vec<String> = conn.smembers(&ai_tool_key).await?;
        let mut projects = Vec::new();

        for project_id in project_ids {
            if let Some(project) = self.get_project(&project_id).await? {
                projects.push(project);
            }
        }

        Ok(projects)
    }

    pub async fn invalidate_project(
        &self,
        project_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let key = keys::project_key(project_id);
        let _: () = conn.del(&key).await?;
        Ok(())
    }
}

/// Session cache operations
pub struct SessionCache {
    pool: RedisPool,
}

impl SessionCache {
    pub fn new(pool: RedisPool) -> Self {
        Self { pool }
    }

    pub async fn store_session(&self, session: &Session) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let key = keys::session_key(&session.id);
        let data = serde_json::to_string(session)?;

        // Store session data
        let _: () = conn.set_ex(&key, &data, 3600).await?; // 1 hour TTL

        // Add to project sessions list
        let project_sessions_key = keys::project_sessions_key(&session.project_id);
        let score = session.updated_at.timestamp();
        let _: () = conn.zadd(&project_sessions_key, &session.id, score).await?;

        Ok(())
    }

    pub async fn get_session(
        &self,
        session_id: &str,
    ) -> Result<Option<Session>, Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let key = keys::session_key(session_id);

        let data: Option<String> = conn.get(&key).await?;
        match data {
            Some(json) => {
                let session: Session = serde_json::from_str(&json)?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    pub async fn get_sessions_by_project(
        &self,
        project_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<Session>, Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let project_sessions_key = keys::project_sessions_key(project_id);

        let limit = limit.unwrap_or(50);
        let session_ids: Vec<String> = conn
            .zrevrange(&project_sessions_key, 0, (limit - 1) as isize)
            .await?;

        let mut sessions = Vec::new();
        for session_id in session_ids {
            if let Some(session) = self.get_session(&session_id).await? {
                sessions.push(session);
            }
        }

        Ok(sessions)
    }

    pub async fn invalidate_session(
        &self,
        session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;
        let key = keys::session_key(session_id);
        let _: () = conn.del(&key).await?;
        Ok(())
    }
}

/// Search and indexing operations
pub struct SearchIndex {
    pool: RedisPool,
}

impl SearchIndex {
    pub fn new(pool: RedisPool) -> Self {
        Self { pool }
    }

    pub async fn index_message(
        &self,
        session_id: &str,
        project_id: &str,
        ai_tool: &AiTool,
        message: &crate::domain::models::Message,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;

        let content = match message {
            crate::domain::models::Message::User { content, .. } => content,
            crate::domain::models::Message::Assistant { content, .. } => content,
            crate::domain::models::Message::Tool { output, .. } => output.as_deref().unwrap_or(""),
            crate::domain::models::Message::System { content, .. } => content,
        };

        let timestamp = match message {
            crate::domain::models::Message::User { timestamp, .. } => *timestamp,
            crate::domain::models::Message::Assistant { timestamp, .. } => *timestamp,
            crate::domain::models::Message::Tool { timestamp, .. } => *timestamp,
            crate::domain::models::Message::System { timestamp, .. } => *timestamp,
        };

        // Create searchable terms (simple word-based indexing)
        let terms = self.extract_search_terms(content);

        // Store in Redis with metadata
        let message_key = format!("msg:{}:{}", session_id, timestamp.timestamp_millis());
        let metadata = serde_json::json!({
            "session_id": session_id,
            "project_id": project_id,
            "ai_tool": ai_tool,
            "content": content,
            "timestamp": timestamp,
        });

        let _: () = conn
            .set_ex(&message_key, metadata.to_string(), 86400)
            .await?; // 24 hours TTL

        // Index terms
        for term in terms {
            let term_key = format!("term:{}", term.to_lowercase());
            let score = timestamp.timestamp() as f64;
            let _: () = conn.zadd(&term_key, &message_key, score).await?;
        }

        Ok(())
    }

    pub async fn search(
        &self,
        query: &SearchQuery,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let mut conn = self.pool.get().await?;

        let terms: Vec<&str> = query.query.split_whitespace().collect();
        let mut result_sets = Vec::new();

        // Get results for each term
        for term in &terms {
            let term_key = format!("term:{}", term.to_lowercase());
            let results: Vec<String> = conn.zrevrange(&term_key, 0, 49).await?; // Top 50 results per term
            result_sets.push(results);
        }

        if result_sets.is_empty() {
            return Ok(Vec::new());
        }

        // Intersect results (simple AND logic)
        let mut intersection = result_sets[0].clone();
        for other_set in &result_sets[1..] {
            intersection.retain(|item| other_set.contains(item));
        }

        // Convert to SearchResult
        let mut search_results = Vec::new();
        for message_key in intersection.into_iter().take(query.limit.unwrap_or(50)) {
            if let Some(metadata_str) = conn.get::<_, Option<String>>(&message_key).await? {
                if let Ok(metadata) = serde_json::from_str::<serde_json::Value>(&metadata_str) {
                    let session_id = metadata["session_id"].as_str().unwrap_or("");
                    let project_id = metadata["project_id"].as_str().unwrap_or("");
                    let content = metadata["content"].as_str().unwrap_or("");
                    let timestamp_str = metadata["timestamp"].as_str().unwrap_or("");
                    let timestamp = DateTime::parse_from_rfc3339(timestamp_str)
                        .unwrap_or_else(|_| Utc::now().into())
                        .with_timezone(&Utc);

                    // Calculate relevance score (simple term frequency)
                    let score = terms
                        .iter()
                        .map(|term| content.to_lowercase().matches(&term.to_lowercase()).count())
                        .sum::<usize>() as f32;

                    search_results.push(SearchResult {
                        session_id: session_id.to_string(),
                        project_id: project_id.to_string(),
                        ai_tool: query.ai_tool.clone().unwrap_or(AiTool::ClaudeCode),
                        message_id: message_key,
                        content_snippet: self.create_snippet(content, &terms),
                        score,
                        timestamp,
                    });
                }
            }
        }

        // Sort by score and timestamp
        search_results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.timestamp.cmp(&a.timestamp))
        });

        Ok(search_results)
    }

    fn extract_search_terms(&self, content: &str) -> Vec<String> {
        content
            .split_whitespace()
            .filter(|word| word.len() > 2) // Skip very short words
            .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|word| !word.is_empty())
            .take(20) // Limit terms per message
            .map(|s| s.to_string())
            .collect()
    }

    fn create_snippet(&self, content: &str, terms: &[&str]) -> String {
        let words: Vec<&str> = content.split_whitespace().collect();
        let max_length = 100;

        // Find first occurrence of any search term
        for (i, word) in words.iter().enumerate() {
            for term in terms {
                if word.to_lowercase().contains(&term.to_lowercase()) {
                    let start = i.saturating_sub(5);
                    let end = (start + 15).min(words.len());
                    let snippet = words[start..end].join(" ");
                    return if snippet.len() > max_length {
                        format!("{}...", &snippet[..max_length])
                    } else {
                        snippet
                    };
                }
            }
        }

        // Fallback: first 100 characters
        if content.len() > max_length {
            format!("{}...", &content[..max_length])
        } else {
            content.to_string()
        }
    }
}

/// Combined cache manager
pub struct CacheManager {
    pub projects: ProjectCache,
    pub sessions: SessionCache,
    pub search: SearchIndex,
}

impl CacheManager {
    pub fn new(pool: RedisPool) -> Self {
        Self {
            projects: ProjectCache::new(pool.clone()),
            sessions: SessionCache::new(pool.clone()),
            search: SearchIndex::new(pool),
        }
    }

    pub async fn clear_all_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: implement clear cache
        Ok(())
    }
}
