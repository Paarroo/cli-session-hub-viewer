// External API clients
// HTTP requests, WebSockets, etc.

use axum::{routing::get, Router, Json};

pub fn api_routes() -> Router<()> {
    Router::new()
        .route("/api/users", get(get_users_api))
}

async fn get_users_api() -> Json<Vec<String>> {
    // For demo, return hardcoded users
    Json(vec!["user1@example.com".to_string(), "user2@example.com".to_string()])
}
// Old boilerplate API disabled (using sea_orm which was removed)
// pub mod api_v1_products;

pub mod sse;
pub mod upload;
// Old routes.rs disabled (used PostgreSQL)
// pub mod routes;

pub use sse::*;
pub use upload::*;
// pub use routes::*;
