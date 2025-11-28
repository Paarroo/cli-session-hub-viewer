//! Standalone API server (without Dioxus frontend)
//! Use this for API-only testing or backend development.
//!
//! Run with: PORT=3003 cargo run --bin server

use axum::{
    routing::{delete, get, post},
    Extension, Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

use cli_session_hub_viewer::handlers::{
    // Projects & Histories (stateless)
    list_projects_handler,
    list_histories_handler,
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
};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Starting CLI Session Hub Viewer API Server (standalone)...");

    // Read port from environment (default: 3001)
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001);

    // Initialize chat handler state
    let chat_state = match ChatHandlerState::new().await {
        Ok(state) => {
            tracing::info!("Chat handler initialized successfully");
            state
        }
        Err(e) => {
            tracing::error!("Failed to initialize chat handler: {}", e);
            std::process::exit(1);
        }
    };

    // Build the application with routes
    // NOTE: Axum 0.8 uses {param} syntax instead of :param
    let app = Router::new()
        // Projects & Histories routes (stateless)
        .route("/api/projects", get(list_projects_handler))
        .route("/api/projects/{encoded_name}/histories", get(list_histories_handler))
        // Conversation details (stateless)
        .route("/api/projects/{encoded_name}/histories/{session_id}", get(get_conversation_handler))
        // Chat routes (with state via Extension)
        .route("/api/chat/native", post(chat_handler))
        .route("/api/chat/status", get(chat_status_handler))
        .route("/api/abort/{request_id}", post(abort_handler))
        .route("/api/sessions/active", get(active_processes_handler))
        .route("/api/sessions/{session_id}", delete(delete_session_handler))
        // Add chat state as Extension (NOT with_state)
        .layer(Extension(chat_state))
        .layer(CorsLayer::permissive());

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
