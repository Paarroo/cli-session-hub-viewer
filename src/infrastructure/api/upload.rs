use axum::{
    Json,
    extract::{Multipart, Path, Request},
    http::{StatusCode, header, HeaderMap},
    response::Response,
    body::Body,
};
use chrono::Utc;
use crate::domain::models::{
    ImageAnalysisRequest, ImageAnalysisResponse, ImageUpload,
    SUPPORTED_IMAGE_TYPES, MAX_IMAGE_SIZE,
};
use std::path::PathBuf;

/// Error response for upload failures
#[derive(serde::Serialize)]
pub struct UploadError {
    pub error: String,
    pub code: String,
}

/// Debug endpoint to see raw headers - helps diagnose multipart issues
/// This endpoint does NOT use Multipart extractor, so it won't reject early
pub async fn debug_upload(headers: HeaderMap, body: axum::body::Bytes) -> String {
    let mut result = String::from("=== DEBUG UPLOAD REQUEST ===\n\n");

    result.push_str("--- HEADERS ---\n");
    for (name, value) in headers.iter() {
        let val_str = value.to_str().unwrap_or("<binary>");
        result.push_str(&format!("{}: {}\n", name, val_str));
        tracing::info!("📤 DEBUG Header: {} = {}", name, val_str);
    }

    result.push_str(&format!("\n--- BODY ---\nLength: {} bytes\n", body.len()));

    // Show first 500 bytes of body as string (for multipart boundary detection)
    let preview_len = std::cmp::min(500, body.len());
    let body_preview = String::from_utf8_lossy(&body[..preview_len]);
    result.push_str(&format!("First {} bytes:\n{}\n", preview_len, body_preview));

    tracing::info!("📤 DEBUG Body length: {} bytes", body.len());
    tracing::info!("📤 DEBUG Body preview:\n{}", body_preview);

    result
}

/// Handle image upload with validation
pub async fn upload_image(headers: HeaderMap, mut multipart: Multipart) -> Result<Json<ImageUpload>, (StatusCode, Json<UploadError>)> {
    // Log all headers for debugging
    tracing::info!("📤 Upload request received - logging headers:");
    for (name, value) in headers.iter() {
        tracing::info!("📤 Header: {} = {:?}", name, value.to_str().unwrap_or("<binary>"));
    }

    // Create uploads directory if it doesn't exist
    // Use absolute path so .exists() check works regardless of CWD
    let upload_dir = std::env::current_dir()
        .map(|cwd| cwd.join("uploads"))
        .unwrap_or_else(|_| PathBuf::from("uploads"));
    std::fs::create_dir_all(&upload_dir).map_err(|e| {
        tracing::error!("Failed to create uploads directory: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(UploadError {
            error: "Failed to create uploads directory".to_string(),
            code: "STORAGE_ERROR".to_string(),
        }))
    })?;

    let field = multipart
        .next_field()
        .await
        .map_err(|e| {
            tracing::warn!("📤 Failed to read multipart field: {}", e);
            (StatusCode::BAD_REQUEST, Json(UploadError {
                error: format!("Invalid multipart request: {}", e),
                code: "INVALID_REQUEST".to_string(),
            }))
        })?
        .ok_or_else(|| {
            tracing::warn!("📤 No file field in request");
            (StatusCode::BAD_REQUEST, Json(UploadError {
                error: "No file provided".to_string(),
                code: "NO_FILE".to_string(),
            }))
        })?;

    let filename = field
        .file_name()
        .ok_or_else(|| {
            tracing::warn!("📤 No filename in field");
            (StatusCode::BAD_REQUEST, Json(UploadError {
                error: "No filename provided".to_string(),
                code: "NO_FILENAME".to_string(),
            }))
        })?
        .to_string();

    // Get content-type from field, or infer from filename extension
    let content_type = field
        .content_type()
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            // Infer from filename extension
            let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
            let inferred = match ext.as_str() {
                "jpg" | "jpeg" => "image/jpeg",
                "png" => "image/png",
                "gif" => "image/gif",
                "webp" => "image/webp",
                _ => "application/octet-stream",
            };
            tracing::info!("📤 No content-type header, inferred from extension: {} -> {}", ext, inferred);
            inferred.to_string()
        });

    tracing::info!("📤 Processing upload: filename={}, content_type={}", filename, content_type);

    // Validate image type against allowed types
    if !SUPPORTED_IMAGE_TYPES.contains(&content_type.as_str()) {
        tracing::warn!("Rejected upload with unsupported type: {}", content_type);
        return Err((StatusCode::BAD_REQUEST, Json(UploadError {
            error: format!(
                "Unsupported image type: {}. Supported: {:?}",
                content_type, SUPPORTED_IMAGE_TYPES
            ),
            code: "UNSUPPORTED_TYPE".to_string(),
        })));
    }

    let data = field.bytes().await.map_err(|e| {
        tracing::warn!("Failed to read file bytes: {}", e);
        (StatusCode::BAD_REQUEST, Json(UploadError {
            error: "Failed to read file data".to_string(),
            code: "READ_ERROR".to_string(),
        }))
    })?;

    let size = data.len();

    // Validate file size
    if size > MAX_IMAGE_SIZE {
        tracing::warn!("Rejected upload: file too large ({} bytes)", size);
        return Err((StatusCode::PAYLOAD_TOO_LARGE, Json(UploadError {
            error: format!(
                "File too large: {} bytes. Maximum: {} bytes ({}MB)",
                size, MAX_IMAGE_SIZE, MAX_IMAGE_SIZE / 1024 / 1024
            ),
            code: "FILE_TOO_LARGE".to_string(),
        })));
    }

    if size == 0 {
        return Err((StatusCode::BAD_REQUEST, Json(UploadError {
            error: "Empty file".to_string(),
            code: "EMPTY_FILE".to_string(),
        })));
    }

    // Generate unique ID and sanitize filename
    let image_id = uuid::Uuid::new_v4().to_string();
    let safe_filename = sanitize_filename(&filename);
    let extension = get_extension_for_type(&content_type);
    let final_filename = format!("{}-{}.{}", image_id, safe_filename, extension);
    let file_path = upload_dir.join(&final_filename);

    // Save file
    std::fs::write(&file_path, &data).map_err(|e| {
        tracing::error!("Failed to write file: {}", e);
        (StatusCode::INTERNAL_SERVER_ERROR, Json(UploadError {
            error: "Failed to save file".to_string(),
            code: "WRITE_ERROR".to_string(),
        }))
    })?;

    let path_str = file_path.to_string_lossy().to_string();
    let url = format!("/api/images/{}", image_id);

    tracing::info!(
        image_id = %image_id,
        filename = %filename,
        size = %size,
        content_type = %content_type,
        "Image uploaded successfully"
    );

    Ok(Json(ImageUpload {
        id: image_id,
        filename: safe_filename,
        content_type,
        size,
        uploaded_at: Utc::now(),
        url: Some(url),
        path: Some(path_str),
    }))
}

/// Sanitize filename to prevent path traversal and special characters
fn sanitize_filename(filename: &str) -> String {
    let stem = std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");

    stem.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .take(50)
        .collect::<String>()
        .to_lowercase()
}

/// Get file extension for content type
fn get_extension_for_type(content_type: &str) -> &'static str {
    match content_type {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "bin",
    }
}

/// Get uploaded image with proper Content-Type
pub async fn get_image(Path(image_id): Path<String>) -> Result<Response, StatusCode> {
    let upload_dir = PathBuf::from("uploads");

    // Validate image_id format (UUID)
    if image_id.len() != 36 || !image_id.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Find file with this ID
    let entries = std::fs::read_dir(&upload_dir).map_err(|_| StatusCode::NOT_FOUND)?;

    for entry in entries.flatten() {
        let filename = entry.file_name().to_string_lossy().to_string();
        if filename.starts_with(&image_id) {
            let data = std::fs::read(entry.path())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Determine content type from extension
            let content_type = if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
                "image/jpeg"
            } else if filename.ends_with(".png") {
                "image/png"
            } else if filename.ends_with(".webp") {
                "image/webp"
            } else if filename.ends_with(".gif") {
                "image/gif"
            } else {
                "application/octet-stream"
            };

            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=86400")
                .body(data.into())
                .unwrap());
        }
    }

    Err(StatusCode::NOT_FOUND)
}

/// Analyze image with Claude CLI
pub async fn analyze_image(
    Json(request): Json<ImageAnalysisRequest>,
) -> Result<Json<ImageAnalysisResponse>, StatusCode> {
    let upload_dir = PathBuf::from("uploads");

    // Find image file
    let entries = std::fs::read_dir(&upload_dir).map_err(|_| StatusCode::NOT_FOUND)?;
    let mut image_path = None;

    for entry in entries.flatten() {
        let filename = entry.file_name().to_string_lossy().to_string();
        if filename.starts_with(&request.image_id) {
            image_path = Some(entry.path());
            break;
        }
    }

    let image_path = image_path.ok_or(StatusCode::NOT_FOUND)?;

    // Call Claude CLI with vision
    let output = tokio::process::Command::new("claude")
        .arg("--image")
        .arg(&image_path)
        .arg(&request.prompt)
        .output()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !output.status.success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let analysis =
        String::from_utf8(output.stdout).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ImageAnalysisResponse {
        analysis,
        model: "claude-3-opus".to_string(),
    }))
}
