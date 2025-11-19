// Shared services
// ApiService is deprecated in fullstack mode - use server functions instead
// Kept for backward compatibility during migration
#[cfg(target_arch = "wasm32")]
pub mod api_service;

#[cfg(target_arch = "wasm32")]
pub use api_service::ApiService;