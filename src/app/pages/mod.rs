pub mod claude_routes;

// Chat page uses WASM-only features (gloo_net, spawn_local)
// TODO: Convert to server functions for fullstack
#[cfg(target_arch = "wasm32")]
pub mod chat;

#[cfg(target_arch = "wasm32")]
pub use chat::ChatPage;
