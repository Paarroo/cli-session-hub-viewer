//! Structured logging module for CLI Session Hub Viewer
//!
//! Provides consistent, contextual logging across the application.
//! Uses tracing spans for operation tracking and structured fields.

use std::path::Path;

/// Log levels for different operations
#[derive(Debug, Clone, Copy)]
pub enum LogOperation {
    ProjectDiscovery,
    SessionCount,
    SessionLoad,
    MessageParsing,
    Grouping,
    PathEncoding,
}

impl LogOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogOperation::ProjectDiscovery => "project_discovery",
            LogOperation::SessionCount => "session_count",
            LogOperation::SessionLoad => "session_load",
            LogOperation::MessageParsing => "message_parsing",
            LogOperation::Grouping => "grouping",
            LogOperation::PathEncoding => "path_encoding",
        }
    }
}

/// Log project discovery start
pub fn log_project_discovery_start(tool: &str) {
    tracing::info!(
        operation = LogOperation::ProjectDiscovery.as_str(),
        ai_tool = tool,
        "Starting project discovery"
    );
}

/// Log project discovery result
pub fn log_project_discovery_result(tool: &str, count: usize, path: &Path) {
    tracing::info!(
        operation = LogOperation::ProjectDiscovery.as_str(),
        ai_tool = tool,
        project_count = count,
        base_path = %path.display(),
        "Project discovery completed"
    );
}

/// Log session count for a project
pub fn log_session_count(tool: &str, project: &str, count: usize) {
    tracing::debug!(
        operation = LogOperation::SessionCount.as_str(),
        ai_tool = tool,
        project = project,
        session_count = count,
        "Counted sessions for project"
    );
}

/// Log session count mismatch (potential bug indicator)
pub fn log_session_count_mismatch(
    tool: &str,
    project: &str,
    expected: usize,
    actual: usize,
) {
    tracing::warn!(
        operation = LogOperation::SessionCount.as_str(),
        ai_tool = tool,
        project = project,
        expected_count = expected,
        actual_count = actual,
        "Session count mismatch detected"
    );
}

/// Log session load attempt
pub fn log_session_load_start(tool: &str, session_id: &str, project: &str) {
    tracing::debug!(
        operation = LogOperation::SessionLoad.as_str(),
        ai_tool = tool,
        session_id = session_id,
        project = project,
        "Loading session"
    );
}

/// Log session load success
pub fn log_session_load_success(tool: &str, session_id: &str, message_count: usize) {
    tracing::info!(
        operation = LogOperation::SessionLoad.as_str(),
        ai_tool = tool,
        session_id = session_id,
        message_count = message_count,
        "Session loaded successfully"
    );
}

/// Log session load failure
pub fn log_session_load_error(tool: &str, session_id: &str, error: &str) {
    tracing::error!(
        operation = LogOperation::SessionLoad.as_str(),
        ai_tool = tool,
        session_id = session_id,
        error = error,
        "Failed to load session"
    );
}

/// Log OpenCode message directory status
pub fn log_opencode_message_dir(session_id: &str, exists: bool, message_count: usize) {
    if exists {
        tracing::debug!(
            operation = LogOperation::MessageParsing.as_str(),
            ai_tool = "opencode",
            session_id = session_id,
            message_dir_exists = true,
            message_count = message_count,
            "OpenCode message directory found"
        );
    } else {
        tracing::warn!(
            operation = LogOperation::MessageParsing.as_str(),
            ai_tool = "opencode",
            session_id = session_id,
            message_dir_exists = false,
            "OpenCode message directory not found - session may be empty"
        );
    }
}

/// Log Gemini directory filtering
pub fn log_gemini_dir_filter(dir_name: &str, reason: &str, skipped: bool) {
    if skipped {
        tracing::debug!(
            operation = LogOperation::ProjectDiscovery.as_str(),
            ai_tool = "gemini",
            directory = dir_name,
            reason = reason,
            "Skipped non-hash Gemini directory"
        );
    }
}

/// Log grouping operation
pub fn log_grouping_start(file_count: usize) {
    tracing::debug!(
        operation = LogOperation::Grouping.as_str(),
        input_files = file_count,
        "Starting conversation grouping"
    );
}

/// Log grouping result
pub fn log_grouping_result(input_count: usize, output_count: usize, duplicates_removed: usize) {
    tracing::info!(
        operation = LogOperation::Grouping.as_str(),
        input_files = input_count,
        unique_sessions = output_count,
        duplicates_removed = duplicates_removed,
        "Conversation grouping completed"
    );
}

/// Log empty session skip
pub fn log_empty_session_skip(tool: &str, session_id: &str, project: &str) {
    tracing::debug!(
        operation = LogOperation::SessionCount.as_str(),
        ai_tool = tool,
        session_id = session_id,
        project = project,
        "Skipped empty session (0 messages)"
    );
}

/// Log project skip (0 sessions)
pub fn log_empty_project_skip(tool: &str, project: &str) {
    tracing::debug!(
        operation = LogOperation::ProjectDiscovery.as_str(),
        ai_tool = tool,
        project = project,
        "Skipped empty project (0 sessions)"
    );
}

/// Log path encoding/decoding
pub fn log_path_operation(operation: &str, input: &str, output: &str) {
    tracing::trace!(
        operation = LogOperation::PathEncoding.as_str(),
        path_operation = operation,
        input = input,
        output = output,
        "Path encoding/decoding"
    );
}

/// Macro for creating structured log context
#[macro_export]
macro_rules! log_context {
    ($tool:expr, $project:expr) => {
        tracing::info_span!(
            "session_hub",
            ai_tool = $tool,
            project = $project
        )
    };
    ($tool:expr, $project:expr, $session:expr) => {
        tracing::info_span!(
            "session_hub",
            ai_tool = $tool,
            project = $project,
            session_id = $session
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_operation_as_str() {
        assert_eq!(LogOperation::ProjectDiscovery.as_str(), "project_discovery");
        assert_eq!(LogOperation::SessionCount.as_str(), "session_count");
        assert_eq!(LogOperation::SessionLoad.as_str(), "session_load");
        assert_eq!(LogOperation::MessageParsing.as_str(), "message_parsing");
        assert_eq!(LogOperation::Grouping.as_str(), "grouping");
        assert_eq!(LogOperation::PathEncoding.as_str(), "path_encoding");
    }
}
