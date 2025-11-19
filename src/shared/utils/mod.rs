// Utility functions
// Formatting, validation, helpers

#[cfg(any(target_arch = "wasm32", target_arch = "wasm32"))]
pub mod stream_parser;

#[cfg(any(target_arch = "wasm32", target_arch = "wasm32"))]
pub use stream_parser::{process_claude_sdk_message, process_stream_line};
