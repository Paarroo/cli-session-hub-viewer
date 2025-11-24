//! File utilities for history parsing
//!
//! Common functions for timestamp extraction, file counting, and session ID parsing

use std::fs;
use std::path::Path;
use std::time::SystemTime;
use chrono::{DateTime, Utc};

/// Get file modification time as RFC3339 string, with fallback to current time
pub fn get_file_mtime_fallback(path: &Path) -> String {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(|t| DateTime::<Utc>::from(t).to_rfc3339())
        .unwrap_or_else(|| Utc::now().to_rfc3339())
}

/// Get the maximum of an internal timestamp and file modification time
/// Returns the more recent timestamp for accurate "time ago" display
pub fn get_max_timestamp(internal_ts: Option<String>, file_mtime: &str) -> String {
    match internal_ts {
        Some(ts) if ts.as_str() > file_mtime => ts,
        Some(_) | None => file_mtime.to_string(),
    }
}

/// Extract session ID from filename by removing prefix and suffix
pub fn extract_session_id(filename: &str, prefix: &str, suffix: &str) -> String {
    filename.trim_start_matches(prefix)
        .trim_end_matches(suffix)
        .to_string()
}

/// Count directory entries matching a predicate
pub fn count_entries_matching<F>(dir: &Path, predicate: F) -> usize
where
    F: Fn(&fs::DirEntry) -> bool,
{
    fs::read_dir(dir)
        .ok()
        .map(|entries| entries.flatten().filter(predicate).count())
        .unwrap_or(0)
}

/// Update latest time if new_time is more recent
pub fn update_latest_time(latest: &mut Option<SystemTime>, new_time: SystemTime) {
    *latest = Some(match *latest {
        None => new_time,
        Some(prev) if new_time > prev => new_time,
        Some(prev) => prev,
    });
}

/// Convert SystemTime to RFC3339 string
pub fn system_time_to_rfc3339(time: SystemTime) -> String {
    DateTime::<Utc>::from(time).to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_session_id() {
        assert_eq!(
            extract_session_id("ses_abc123.json", "ses_", ".json"),
            "abc123"
        );
        assert_eq!(
            extract_session_id("session-xyz.json", "session-", ".json"),
            "xyz"
        );
    }

    #[test]
    fn test_get_max_timestamp() {
        let file_time = "2024-01-01T12:00:00Z";

        // Internal timestamp is newer
        assert_eq!(
            get_max_timestamp(Some("2024-01-02T12:00:00Z".to_string()), file_time),
            "2024-01-02T12:00:00Z"
        );

        // File time is newer
        assert_eq!(
            get_max_timestamp(Some("2023-12-01T12:00:00Z".to_string()), file_time),
            file_time
        );

        // No internal timestamp
        assert_eq!(
            get_max_timestamp(None, file_time),
            file_time
        );
    }
}
