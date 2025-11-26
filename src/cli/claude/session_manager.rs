//! Session manager for Claude CLI
//!
//! Manages conversation sessions, tracks active processes, and handles abort requests.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use super::executor::{ClaudeProcess, ExecutorError};

/// Unique identifier for a request (for abort handling)
pub type RequestId = String;

/// Unique identifier for a session (for conversation continuity)
pub type SessionId = String;

/// Session information
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// Session ID from Claude CLI
    pub session_id: SessionId,
    /// Working directory for this session
    pub working_directory: Option<String>,
    /// Number of messages in this session
    pub message_count: u32,
    /// Last activity timestamp
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

impl SessionInfo {
    pub fn new(session_id: SessionId) -> Self {
        Self {
            session_id,
            working_directory: None,
            message_count: 0,
            last_activity: chrono::Utc::now(),
        }
    }

    pub fn with_working_directory(mut self, dir: String) -> Self {
        self.working_directory = Some(dir);
        self
    }

    pub fn increment_message_count(&mut self) {
        self.message_count += 1;
        self.last_activity = chrono::Utc::now();
    }
}

/// Active process entry for abort handling
#[allow(dead_code)]
struct ActiveProcess {
    process: ClaudeProcess,
    #[allow(dead_code)]
    session_id: Option<SessionId>,
}

/// Session manager for tracking sessions and active processes
pub struct SessionManager {
    /// Active processes by request ID (for abort)
    active_processes: Arc<Mutex<HashMap<RequestId, ActiveProcess>>>,
    /// Session information by session ID
    sessions: Arc<RwLock<HashMap<SessionId, SessionInfo>>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            active_processes: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an active process for a request
    pub async fn register_process(
        &self,
        request_id: RequestId,
        process: ClaudeProcess,
        session_id: Option<SessionId>,
    ) {
        let mut processes = self.active_processes.lock().await;
        processes.insert(
            request_id,
            ActiveProcess {
                process,
                session_id,
            },
        );
    }

    /// Unregister a process when it completes
    pub async fn unregister_process(&self, request_id: &RequestId) -> Option<ClaudeProcess> {
        let mut processes = self.active_processes.lock().await;
        processes.remove(request_id).map(|entry| entry.process)
    }

    /// Abort a running process by request ID
    pub async fn abort_process(&self, request_id: &RequestId) -> Result<(), ExecutorError> {
        let mut processes = self.active_processes.lock().await;
        if let Some(mut entry) = processes.remove(request_id) {
            entry.process.kill().await?;
            tracing::info!("Aborted process for request {}", request_id);
            Ok(())
        } else {
            tracing::warn!("No active process found for request {}", request_id);
            Err(ExecutorError::ProcessError(format!(
                "No active process for request {}",
                request_id
            )))
        }
    }

    /// Check if a request has an active process
    pub async fn is_process_active(&self, request_id: &RequestId) -> bool {
        let processes = self.active_processes.lock().await;
        processes.contains_key(request_id)
    }

    /// Get or create a session
    pub async fn get_or_create_session(&self, session_id: SessionId) -> SessionInfo {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(&session_id) {
            return session.clone();
        }
        drop(sessions);

        let mut sessions = self.sessions.write().await;
        let session = SessionInfo::new(session_id.clone());
        sessions.insert(session_id, session.clone());
        session
    }

    /// Update session info
    pub async fn update_session(&self, session_id: &SessionId, update_fn: impl FnOnce(&mut SessionInfo)) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            update_fn(session);
        }
    }

    /// Get session info
    pub async fn get_session(&self, session_id: &SessionId) -> Option<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// List all sessions
    pub async fn list_sessions(&self) -> Vec<SessionInfo> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Remove a session
    pub async fn remove_session(&self, session_id: &SessionId) -> Option<SessionInfo> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id)
    }

    /// Cleanup old sessions (older than specified duration)
    pub async fn cleanup_old_sessions(&self, max_age: chrono::Duration) {
        let now = chrono::Utc::now();
        let mut sessions = self.sessions.write().await;
        sessions.retain(|_, session| {
            now.signed_duration_since(session.last_activity) < max_age
        });
    }

    /// Get count of active processes
    pub async fn active_process_count(&self) -> usize {
        let processes = self.active_processes.lock().await;
        processes.len()
    }
}

/// Global session manager instance
static SESSION_MANAGER: std::sync::OnceLock<SessionManager> = std::sync::OnceLock::new();

/// Get the global session manager
pub fn get_session_manager() -> &'static SessionManager {
    SESSION_MANAGER.get_or_init(SessionManager::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let manager = SessionManager::new();
        let session = manager.get_or_create_session("test-123".to_string()).await;
        assert_eq!(session.session_id, "test-123");
        assert_eq!(session.message_count, 0);
    }

    #[tokio::test]
    async fn test_session_update() {
        let manager = SessionManager::new();
        manager.get_or_create_session("test-456".to_string()).await;

        manager
            .update_session(&"test-456".to_string(), |s| {
                s.increment_message_count();
                s.working_directory = Some("/tmp".to_string());
            })
            .await;

        let session = manager.get_session(&"test-456".to_string()).await.unwrap();
        assert_eq!(session.message_count, 1);
        assert_eq!(session.working_directory, Some("/tmp".to_string()));
    }
}
