// Public API exports (shared between client and server)
pub mod domain;
pub mod shared;
pub mod config;

// App is always available in fullstack mode
pub mod app;

// Server functions (available on both client and server in fullstack)
pub mod server_fns;

// Server-only modules (NOT compiled for WASM)
#[cfg(not(target_arch = "wasm32"))]
pub mod infrastructure;
#[cfg(not(target_arch = "wasm32"))]
pub mod handlers;
#[cfg(not(target_arch = "wasm32"))]
pub mod history;
#[cfg(not(target_arch = "wasm32"))]
pub mod cli;
