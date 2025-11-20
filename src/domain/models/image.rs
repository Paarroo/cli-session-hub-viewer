use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Supported image formats
pub const SUPPORTED_IMAGE_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/webp",
    "image/gif",
];

/// Maximum image size in bytes (10MB)
pub const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;

/// Maximum number of images per message (matches claude.ai limit)
/// Note: API supports up to 100, but claude.ai/Claude Code uses 20
pub const MAX_IMAGES_PER_MESSAGE: usize = 20;

/// Image attachment for chat messages
/// Contains both URL for display and path for CLI integration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageAttachment {
    pub id: String,
    pub url: String,           // URL for frontend display (/api/images/{id})
    pub path: String,          // Server filesystem path for CLI
    pub filename: String,
    pub media_type: String,
    pub size_bytes: usize,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

/// Image upload response (extended)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageUpload {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size: usize,
    pub uploaded_at: DateTime<Utc>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

/// Pending image (before upload completes)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingImage {
    pub id: String,
    pub filename: String,
    pub preview_url: String,  // Object URL for preview
    pub size_bytes: usize,
    pub upload_progress: f32, // 0.0 to 1.0
    pub error: Option<String>,
}

/// Image analysis request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageAnalysisRequest {
    pub image_id: String,
    pub prompt: String,
}

/// Image analysis response
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageAnalysisResponse {
    pub analysis: String,
    pub model: String,
}
