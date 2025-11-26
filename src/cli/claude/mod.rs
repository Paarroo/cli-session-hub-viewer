//! Claude CLI interaction module
//!
//! This module provides functionality to interact with Claude Code CLI,
//! similar to the TypeScript implementation in claude-code-webui.

pub mod detection;

pub mod executor;

pub mod session_manager;

pub use detection::*;

pub use executor::*;

pub use session_manager::*;
