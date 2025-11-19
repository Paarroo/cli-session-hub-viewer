//! Image upload hook for managing image attachments in chat
//!
//! Handles file selection, preview generation, upload to server,
//! and state management for pending/uploaded images.

use dioxus::prelude::*;
use crate::domain::models::{
    ImageUpload, PendingImage, ImageAttachment,
    SUPPORTED_IMAGE_TYPES, MAX_IMAGE_SIZE, MAX_IMAGES_PER_MESSAGE,
};

/// Image upload state and operations
#[derive(Clone, PartialEq)]
pub struct ImageUploadState {
    /// Images currently being uploaded or pending
    pub pending_images: Signal<Vec<PendingImage>>,
    /// Successfully uploaded images ready to send
    pub uploaded_images: Signal<Vec<ImageAttachment>>,
    /// Error message if any
    pub error: Signal<Option<String>>,
    /// Whether upload is in progress
    pub is_uploading: Signal<bool>,
}

impl ImageUploadState {
    /// Check if we can add more images
    pub fn can_add_more(&self) -> bool {
        let pending_count = self.pending_images.read().len();
        let uploaded_count = self.uploaded_images.read().len();
        (pending_count + uploaded_count) < MAX_IMAGES_PER_MESSAGE
    }

    /// Get total image count
    pub fn total_count(&self) -> usize {
        self.pending_images.read().len() + self.uploaded_images.read().len()
    }

    /// Check if there are any images
    pub fn has_images(&self) -> bool {
        !self.pending_images.read().is_empty() || !self.uploaded_images.read().is_empty()
    }

    /// Clear all images
    pub fn clear(&mut self) {
        self.pending_images.write().clear();
        self.uploaded_images.write().clear();
        self.error.set(None);
    }

    /// Clear error
    pub fn clear_error(&mut self) {
        self.error.set(None);
    }

    /// Remove a pending image by ID
    pub fn remove_pending(&mut self, id: &str) {
        self.pending_images.write().retain(|img| img.id != id);
    }

    /// Remove an uploaded image by ID
    pub fn remove_uploaded(&mut self, id: &str) {
        self.uploaded_images.write().retain(|img| img.id != id);
    }

    /// Get all image paths for sending to backend
    pub fn get_image_paths(&self) -> Vec<String> {
        self.uploaded_images
            .read()
            .iter()
            .map(|img| img.path.clone())
            .collect()
    }

    /// Validate a file before upload
    pub fn validate_file(&self, content_type: &str, size: usize) -> Result<(), String> {
        // Check type
        if !SUPPORTED_IMAGE_TYPES.contains(&content_type) {
            return Err(format!(
                "Type non supporté: {}. Acceptés: JPEG, PNG, WebP, GIF",
                content_type
            ));
        }

        // Check size
        if size > MAX_IMAGE_SIZE {
            return Err(format!(
                "Fichier trop volumineux: {} MB. Maximum: {} MB",
                size / 1024 / 1024,
                MAX_IMAGE_SIZE / 1024 / 1024
            ));
        }

        // Check count
        if !self.can_add_more() {
            return Err(format!(
                "Maximum {} images par message",
                MAX_IMAGES_PER_MESSAGE
            ));
        }

        Ok(())
    }

    /// Add a pending image (before upload starts)
    pub fn add_pending(&mut self, id: String, filename: String, preview_url: String, size: usize) {
        self.pending_images.write().push(PendingImage {
            id,
            filename,
            preview_url,
            size_bytes: size,
            upload_progress: 0.0,
            error: None,
        });
    }

    /// Update upload progress for a pending image
    pub fn update_progress(&mut self, id: &str, progress: f32) {
        if let Some(img) = self.pending_images.write().iter_mut().find(|i| i.id == id) {
            img.upload_progress = progress;
        }
    }

    /// Mark pending image as failed
    pub fn mark_failed(&mut self, id: &str, error: String) {
        if let Some(img) = self.pending_images.write().iter_mut().find(|i| i.id == id) {
            img.error = Some(error);
        }
    }

    /// Convert pending to uploaded after successful upload
    pub fn complete_upload(&mut self, pending_id: &str, upload_response: ImageUpload) {
        tracing::info!("📸 complete_upload called for pending_id: {}", pending_id);
        tracing::info!("📸 Response: url={:?}, path={:?}", upload_response.url, upload_response.path);

        // Remove from pending
        self.pending_images.write().retain(|img| img.id != pending_id);

        // Add to uploaded
        if let (Some(url), Some(path)) = (upload_response.url, upload_response.path) {
            tracing::info!("📸 Adding to uploaded_images: {} -> {}", upload_response.filename, path);
            self.uploaded_images.write().push(ImageAttachment {
                id: upload_response.id,
                url,
                path,
                filename: upload_response.filename,
                media_type: upload_response.content_type,
                size_bytes: upload_response.size,
                width: None,
                height: None,
            });
            tracing::info!("📸 uploaded_images count now: {}", self.uploaded_images.read().len());
        } else {
            tracing::warn!("📸 Missing url or path in upload response!");
        }
    }
}

/// Hook to manage image uploads
pub fn use_image_upload() -> ImageUploadState {
    let pending_images = use_signal(Vec::<PendingImage>::new);
    let uploaded_images = use_signal(Vec::<ImageAttachment>::new);
    let error = use_signal(|| None::<String>);
    let is_uploading = use_signal(|| false);

    ImageUploadState {
        pending_images,
        uploaded_images,
        error,
        is_uploading,
    }
}

/// Upload a file to the server
/// This is a standalone function that can be called from components
#[cfg(target_arch = "wasm32")]
pub async fn upload_file_to_server(
    file: web_sys::File,
    state: &mut ImageUploadState,
) -> Result<ImageUpload, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{FormData, RequestInit, Request, Response};

    let filename = file.name();
    let content_type = file.type_();
    let size = file.size() as usize;

    // Validate file
    state.validate_file(&content_type, size)?;

    // Generate temporary ID for tracking
    let temp_id = uuid::Uuid::new_v4().to_string();

    // Create object URL for preview
    let preview_url = web_sys::Url::create_object_url_with_blob(&file)
        .map_err(|_| "Échec création preview".to_string())?;

    // Add to pending
    state.add_pending(temp_id.clone(), filename.clone(), preview_url.clone(), size);
    state.is_uploading.set(true);

    // Create FormData
    let form_data = FormData::new()
        .map_err(|_| "Échec création FormData".to_string())?;

    form_data
        .append_with_blob_and_filename("file", &file, &filename)
        .map_err(|_| "Échec ajout fichier".to_string())?;

    web_sys::console::log_1(&format!("[WASM] FormData created for: {}, type: {}", filename, content_type).into());

    // Create fetch options
    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.body(Some(&form_data));
    // IMPORTANT: Do NOT set Content-Type manually - browser will add it with boundary

    // DEBUG: First send to debug endpoint to see what the server receives
    web_sys::console::log_1(&"[WASM] ===== SENDING TO DEBUG ENDPOINT FIRST =====".into());
    {
        let mut debug_opts = RequestInit::new();
        debug_opts.method("POST");
        debug_opts.body(Some(&form_data));

        let debug_request = Request::new_with_str_and_init("/api/upload-debug", &debug_opts)
            .map_err(|e| format!("Debug request failed: {:?}", e))?;

        let window = web_sys::window().ok_or("Pas de window")?;
        let debug_resp_value = JsFuture::from(window.fetch_with_request(&debug_request))
            .await
            .map_err(|e| format!("Debug fetch failed: {:?}", e))?;

        let debug_resp: Response = debug_resp_value.dyn_into().map_err(|_| "Invalid response")?;
        web_sys::console::log_1(&format!("[WASM] DEBUG Response status: {}", debug_resp.status()).into());

        if let Ok(text_promise) = debug_resp.text() {
            if let Ok(text_value) = JsFuture::from(text_promise).await {
                if let Some(text) = text_value.as_string() {
                    web_sys::console::log_1(&format!("[WASM] DEBUG Response:\n{}", text).into());
                }
            }
        }
    }
    web_sys::console::log_1(&"[WASM] ===== END DEBUG, NOW REAL UPLOAD =====".into());

    // Need to recreate FormData since it was consumed
    let form_data = FormData::new()
        .map_err(|_| "Échec création FormData".to_string())?;
    form_data
        .append_with_blob_and_filename("file", &file, &filename)
        .map_err(|_| "Échec ajout fichier".to_string())?;

    let mut opts = RequestInit::new();
    opts.method("POST");
    opts.body(Some(&form_data));

    let request = Request::new_with_str_and_init("/api/upload", &opts)
        .map_err(|e| format!("Échec création requête: {:?}", e))?;

    web_sys::console::log_1(&"[WASM] Sending to real upload endpoint...".into());

    let window = web_sys::window().ok_or("Pas de window")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| {
            web_sys::console::error_1(&format!("[WASM] Fetch error: {:?}", e).into());
            format!("Échec requête réseau: {:?}", e)
        })?;

    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| "Réponse invalide".to_string())?;

    let status = resp.status();
    web_sys::console::log_1(&format!("[WASM] Upload Response status: {}", status).into());

    if !resp.ok() {
        // Try to get error body
        let error_text = match JsFuture::from(resp.text().map_err(|_| "No text")?).await {
            Ok(text) => text.as_string().unwrap_or_else(|| "Unknown error".to_string()),
            Err(_) => "Unknown error".to_string(),
        };
        web_sys::console::error_1(&format!("[WASM] Upload error {}: {}", status, error_text).into());
        state.mark_failed(&temp_id, format!("Erreur serveur: {}", status));
        state.is_uploading.set(false);
        return Err(format!("Erreur serveur: {} - {}", status, error_text));
    }

    // Parse JSON response
    let json = JsFuture::from(resp.json().map_err(|_| "Pas de JSON")?)
        .await
        .map_err(|_| "Échec parse JSON".to_string())?;

    let upload: ImageUpload = serde_wasm_bindgen::from_value(json)
        .map_err(|e| format!("Échec désérialisation: {:?}", e))?;

    // Complete upload
    web_sys::console::log_1(&format!("[WASM] Upload complete, calling complete_upload for {}", temp_id).into());
    state.complete_upload(&temp_id, upload.clone());
    state.is_uploading.set(false);
    web_sys::console::log_1(&format!("[WASM] After complete_upload, uploaded_images count: {}", state.uploaded_images.read().len()).into());

    // Cleanup preview URL
    let _ = web_sys::Url::revoke_object_url(&preview_url);

    Ok(upload)
}

/// Server-side stub (no-op)
#[cfg(not(target_arch = "wasm32"))]
pub async fn upload_file_to_server(
    _file: (),
    _state: &mut ImageUploadState,
) -> Result<ImageUpload, String> {
    Err("Upload only available in browser".to_string())
}
