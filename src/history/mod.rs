pub mod types;
pub mod parser;
pub mod parsers;
pub mod path_utils;
pub mod timestamp_restore;
pub mod grouping;
pub mod discovery;
pub mod file_utils;

// Core types (from types.rs)
pub use types::{
    ConversationHistory,
    Message,
    ProjectInfo,
    ContentBlock,
    RawHistoryLine,
    InnerMessage,
    MessageContent,
    ConversationFile,
    ClaudeJsonlEntry,
    InnerContentBlock,
    HistorySnapshot,
};

// Parsing functions
pub use parser::{
    parse_jsonl_file,
    parse_history_file_metadata,
    list_projects,
    list_project_histories,
    list_project_summaries,
    list_project_summaries_for_tool,
    load_conversation,
};

// Discovery functions
pub use discovery::discover_projects;

// Path utilities
pub use path_utils::{encode_project_path, decode_project_path, get_claude_projects_dir};

// Grouping (for deduplication)
pub use grouping::{ConversationSummary, group_conversations};

// Timestamp restoration
pub use timestamp_restore::{
    restore_timestamps,
    sort_by_timestamp,
    process_conversation_messages,
    ConversationMetadata,
};
