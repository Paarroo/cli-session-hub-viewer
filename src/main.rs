//! CLI Session Hub Viewer - Main Entry Point
//!
//! This file configures the server with Axum routes and Dioxus application.
//! Uses dioxus::serve() pattern for dx serve compatibility.

use cli_session_hub_viewer::app::App;

// Server entry point - NO #[tokio::main], dioxus::serve() creates its own runtime
#[cfg(feature = "server")]
fn main() {
    // IMPORTANT: Use dioxus::server::axum, NOT axum directly
    use dioxus::server::axum::{routing::{delete, get, post}, Extension, extract::DefaultBodyLimit};

    // Set panic hook to print full backtrace
    std::panic::set_hook(Box::new(|panic_info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        eprintln!("\n=== PANIC CAUGHT ===");
        eprintln!("Panic info: {}", panic_info);
        eprintln!("Backtrace:\n{}", backtrace);
        eprintln!("=== END PANIC ===\n");
    }));

    // Initialize tracing BEFORE dioxus::serve
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting CLI Session Hub Viewer...");

    use cli_session_hub_viewer::handlers::{
        // Projects & Histories (no state required)
        list_projects_handler,
        list_histories_handler,
        get_active_session_handler,
        // Conversation details
        get_conversation_handler,
        // Chat
        chat_handler,
        chat_status_handler,
        ChatHandlerState,
        // Abort & Sessions
        abort_handler,
        active_processes_handler,
        delete_session_handler,
        // SSE for real-time sync
        sse_session_handler,
    };

    // Image upload handlers
    use cli_session_hub_viewer::infrastructure::api::upload::{upload_image, get_image, debug_upload};

    // NO #[tokio::main] - dioxus::serve creates its own runtime
    dioxus::serve(|| {
        async move {
            // Async initialization inside the closure
            let chat_state_result = ChatHandlerState::new().await;

            // Get the base Dioxus router
            // NOTE: Axum 0.8 uses {param} syntax instead of :param
            let mut router = dioxus::server::router(App)
                // Projects & Histories routes (stateless)
                .route("/api/projects", get(list_projects_handler))
                .route("/api/projects/{encoded_name}/histories", get(list_histories_handler))
                .route("/api/projects/{encoded_name}/active-session", get(get_active_session_handler))
                // Conversation details (stateless)
                .route("/api/projects/{encoded_name}/histories/{session_id}", get(get_conversation_handler))
                // SSE endpoint for real-time CLI → Web sync
                .route("/api/sse/{encoded_name}/{session_id}", get(sse_session_handler))
                // Image upload routes - with increased body limit (10MB for images)
                .route("/api/upload", post(upload_image))
                .route("/api/upload-debug", post(debug_upload))  // Debug endpoint without Multipart extractor
                .route("/api/images/{image_id}", get(get_image))
                .layer(DefaultBodyLimit::max(10 * 1024 * 1024));  // 10MB = MAX_IMAGE_SIZE

            // Add chat routes only if chat state was initialized successfully
            match chat_state_result {
                Ok(chat_state) => {
                    tracing::info!("Chat handler state initialized successfully - chat features enabled");
                    router = router
                        // Chat routes (with state via Extension)
                        .route("/api/chat/native", post(chat_handler))
                        .route("/api/chat/status", get(chat_status_handler))
                        .route("/api/abort/{request_id}", post(abort_handler))
                        .route("/api/sessions/active", get(active_processes_handler))
                        .route("/api/sessions/{session_id}", delete(delete_session_handler))
                        // Add chat state as Extension
                        .layer(Extension(chat_state));
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to initialize chat state: {}. Chat features disabled, but history viewing works.",
                        e
                    );
                }
            }

            Ok(router)
        }
    });
}

// WASM entry point (browser) - no server feature
#[cfg(all(not(feature = "server"), target_arch = "wasm32"))]
fn main() {
    // Log to browser console to confirm WASM loaded
    web_sys::console::log_1(&"[WASM] CLI Session Hub Viewer - WASM initialized!".into());
    dioxus::launch(App);
}

// Native client (desktop) - no server feature, not WASM
#[cfg(all(not(feature = "server"), not(target_arch = "wasm32")))]
fn main() {
    dioxus::launch(App);
}
