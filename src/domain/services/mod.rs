// Business logic services
// Framework-agnostic, 100% testable

// OpenCode project/session discovery (actual parsing in history/parsers/opencode.rs)
pub mod opencode_parser;
pub use opencode_parser::*;

// Process bridges (Server-only - use tokio for process spawning)
#[cfg(not(target_arch = "wasm32"))]
pub mod opencode_bridge;
#[cfg(not(target_arch = "wasm32"))]
pub mod gemini_bridge;
#[cfg(not(target_arch = "wasm32"))]
pub mod claude_process;

#[cfg(not(target_arch = "wasm32"))]
pub use opencode_bridge::*;
#[cfg(not(target_arch = "wasm32"))]
pub use gemini_bridge::*;
#[cfg(not(target_arch = "wasm32"))]
pub use claude_process::*;
