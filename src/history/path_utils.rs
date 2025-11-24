use std::path::PathBuf;

/// Encode a filesystem path to Claude's dash-separated format
/// Example: "/Users/toto/project" -> "-Users-toto-project"
/// This matches the format used by Claude CLI in ~/.claude/projects/
pub fn encode_project_path(path: &str) -> String {
    // Remove trailing slashes and replace special characters with dashes
    let normalized = path.trim_end_matches('/');
    // Replace /, \, :, ., _ with -
    let mut encoded = String::with_capacity(normalized.len() + 1);
    for c in normalized.chars() {
        match c {
            '/' | '\\' | ':' | '.' | '_' => encoded.push('-'),
            _ => encoded.push(c),
        }
    }
    encoded
}

/// Decode path from directory name (dash-separated format)
/// Example: "-Users-toto-project" -> "/Users/toto/project"
/// WARNING: This function is lossy - it converts ALL dashes to slashes.
/// For project name extraction, use smart_decode_project_path instead.
pub fn decode_project_path(encoded: &str) -> Result<String, String> {
    // The dash-separated format: "-Users-toto-Desktop-Code"
    if encoded.starts_with('-') {
        // Convert dashes to slashes, remove leading dash
        let path = encoded.trim_start_matches('-').replace('-', "/");
        return Ok(format!("/{}", path));
    }

    // If it doesn't start with '-', assume it's already decoded or a simple name
    Ok(encoded.to_string())
}

/// Smart decode that validates path segments against filesystem
/// Returns (decoded_path, project_name) tuple
/// Example: "-Users-toto-Desktop-Code-my-project" -> ("/Users/toto/Desktop/Code/my-project", "my-project")
///
/// Algorithm:
/// 1. Split encoded string by '-' (after removing leading dash)
/// 2. Progressively build path, validating each segment exists on filesystem
/// 3. When a segment doesn't exist, try to combine remaining segments as project name
/// 4. Project name preserves original dashes
pub fn smart_decode_project_path(encoded: &str) -> (String, String) {
    // Handle non-encoded paths
    if !encoded.starts_with('-') {
        return (encoded.to_string(), encoded.to_string());
    }

    let without_leading = encoded.trim_start_matches('-');
    let segments: Vec<&str> = without_leading.split('-').collect();

    if segments.is_empty() {
        return (encoded.to_string(), encoded.to_string());
    }

    // Try progressively longer paths until we find where the filesystem path ends
    let mut valid_path = String::new();
    let mut last_valid_index = 0;

    for i in 0..segments.len() {
        let test_path = format!("/{}", segments[..=i].join("/"));
        let path_buf = std::path::Path::new(&test_path);

        if path_buf.exists() {
            valid_path = test_path;
            last_valid_index = i + 1;
        }
    }

    // If we found a valid path, the remaining segments form the project name
    if !valid_path.is_empty() && last_valid_index < segments.len() {
        // Reconstruct project name with dashes preserved
        let project_name = segments[last_valid_index..].join("-");
        let full_path = format!("{}/{}", valid_path, project_name);
        return (full_path, project_name);
    }

    // If the entire path is valid (no project name suffix)
    if !valid_path.is_empty() {
        // Use the last segment as project name
        let project_name = segments.last().unwrap_or(&"unknown").to_string();
        return (valid_path, project_name);
    }

    // Fallback: use full decode (lossy) and extract last segment
    let full_decode = format!("/{}", without_leading.replace('-', "/"));
    let project_name = std::path::Path::new(&full_decode)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    (full_decode, project_name)
}

use crate::domain::models::AiTool;
use sha2::{Sha256, Digest};

/// Decode Gemini project hash to original path
/// Gemini uses SHA256(path) as directory name
/// We try common paths to find a match
/// Returns (decoded_path, project_name) or fallback to hash
pub fn decode_gemini_hash(hash: &str) -> (String, String) {
    let home = std::env::var("HOME").unwrap_or_default();

    // Common base directories to search
    let base_dirs = vec![
        format!("{}/Desktop/Code", home),
        format!("{}/Desktop", home),
        format!("{}/Documents", home),
        format!("{}/Projects", home),
        format!("{}/dev", home),
        format!("{}/src", home),
        format!("{}/code", home),
        home.clone(),
        "/tmp".to_string(),
        "/private/tmp".to_string(),
    ];

    // First check if hash matches any base directory directly
    for base in &base_dirs {
        if compute_sha256(base) == hash {
            let name = std::path::Path::new(base)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Gemini")
                .to_string();
            return (base.clone(), name);
        }
    }

    // Then recursively check subdirectories (1 level deep)
    for base in &base_dirs {
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let path_str = path.to_string_lossy().to_string();
                    if compute_sha256(&path_str) == hash {
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Gemini")
                            .to_string();
                        return (path_str, name);
                    }
                }
            }
        }
    }

    // Fallback: return truncated hash
    let short_hash = if hash.len() > 12 { &hash[..12] } else { hash };
    (hash.to_string(), format!("Gemini-{}", short_hash))
}

/// Compute SHA256 hash of a string (for Gemini path matching)
fn compute_sha256(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Get the base storage directory path for a specific AI tool
/// Returns the root directory where projects/sessions are stored:
/// - Claude: ~/.claude/projects/
/// - OpenCode: ~/.local/share/opencode/storage/session/
/// - Gemini: ~/.gemini/tmp/ (contains {hash}/chats/ subdirs)
pub fn get_projects_dir(ai_tool: &AiTool) -> Result<PathBuf, String> {
    let home = std::env::var("HOME")
        .map_err(|_| "HOME environment variable not set".to_string())?;

    let path = match ai_tool {
        AiTool::ClaudeCode => PathBuf::from(&home).join(".claude").join("projects"),
        AiTool::OpenCode => PathBuf::from(&home)
            .join(".local")
            .join("share")
            .join("opencode")
            .join("storage")
            .join("session")
            .join("global"),
        AiTool::Gemini => PathBuf::from(&home).join(".gemini").join("tmp"),
    };

    Ok(path)
}

/// Get the Claude projects directory path (legacy function)
/// Returns ~/.claude/projects/
pub fn get_claude_projects_dir() -> Result<PathBuf, String> {
    get_projects_dir(&AiTool::ClaudeCode)
}

/// Get the path to a specific project's directory
/// Example: "L1VzZXJzL3RvdG8vcHJvamVjdA" -> ~/.claude/projects/L1VzZXJzL3RvdG8vcHJvamVjdA/
pub fn get_project_dir(encoded_name: &str) -> Result<PathBuf, String> {
    let projects_dir = get_claude_projects_dir()?;
    Ok(projects_dir.join(encoded_name))
}

/// List all project folders in the Claude projects directory
pub fn list_project_folders(projects_dir: &PathBuf) -> Result<Vec<PathBuf>, String> {
    if !projects_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(projects_dir)
        .map_err(|e| format!("Failed to read projects directory: {}", e))?;

    let mut folders = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            folders.push(path);
        }
    }

    Ok(folders)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_produces_dash_format() {
        let path = "/Users/toto/Desktop/Code/my-project";
        let encoded = encode_project_path(path);
        // Should produce dash-separated format like Claude CLI
        assert_eq!(encoded, "-Users-toto-Desktop-Code-my-project");
    }

    #[test]
    fn test_decode_dash_format() {
        // Note: The encoding is lossy - dashes in original paths become slashes when decoded
        // Using a path without dashes in the filename to test the decode logic
        let encoded = "-Users-toto-Desktop-Code-myproject";
        let decoded = decode_project_path(encoded).unwrap();
        assert_eq!(decoded, "/Users/toto/Desktop/Code/myproject");
    }

    #[test]
    fn test_encode_replaces_special_chars() {
        let path = "/Users/toto/my.project_test";
        let encoded = encode_project_path(path);
        // Dots and underscores should be replaced with dashes
        assert_eq!(encoded, "-Users-toto-my-project-test");
    }

    #[test]
    fn test_smart_decode_non_encoded() {
        let (path, name) = smart_decode_project_path("simple-name");
        assert_eq!(path, "simple-name");
        assert_eq!(name, "simple-name");
    }

    #[test]
    fn test_smart_decode_fallback() {
        // When filesystem validation fails, should fall back to lossy decode
        let (path, name) = smart_decode_project_path("-nonexistent-path-my-project");
        assert_eq!(path, "/nonexistent/path/my/project");
        assert_eq!(name, "project");
    }
}
