//! CLI-specific parsers module
//!
//! Each AI tool has its own parser module:
//! - `claude`: Parse Claude Code .jsonl files
//! - `opencode`: Parse OpenCode ses_*.json files
//! - `gemini`: Parse Gemini session-*.json files

pub mod claude;
pub mod opencode;
pub mod gemini;

// Re-export main parsing functions for convenience
pub use claude::parse_claude_summaries;
pub use opencode::parse_opencode_summaries;
pub use gemini::parse_gemini_summaries;
