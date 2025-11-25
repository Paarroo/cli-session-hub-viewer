pub mod projects;
pub mod histories;
pub mod chat_proxy;

/// Native chat handler using Claude CLI directly
pub mod chat;

/// Abort handler for canceling running chat requests
pub mod abort;

/// Conversation management handlers (delete, archive, update)
pub mod conversation;

/// SSE handler for real-time CLI → Web synchronization
pub mod sse;

pub use projects::list_projects_handler;
pub use histories::{list_histories_handler, get_active_session_handler};
pub use chat_proxy::{chat_proxy_handler, abort_proxy_handler};

pub use chat::{chat_handler, chat_status_handler, ChatHandlerState, ChatRequest};

pub use abort::{abort_handler, active_processes_handler, delete_session_handler};

pub use conversation::{
    delete_conversation_handler, archive_conversation_handler,
    update_conversation_handler, get_conversation_handler,
    ConversationHandlerState,
};

pub use sse::sse_session_handler;
