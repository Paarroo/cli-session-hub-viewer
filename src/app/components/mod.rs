pub mod button;
pub mod card;
pub mod common;
pub mod claude_viewer;
pub mod message_item;
pub mod permission_dialog;
pub mod plan_mode_dialog;
pub mod theme_toggle;
pub mod theme_selector;
pub mod ai_tool_selector;
pub mod cli_selector;
pub mod delete_button;
pub mod image_upload;

// Chat input - available on all platforms for SSR + hydration
pub mod chat_input;

// Message rendering
pub use message_item::{MessageItem, MessageInputPlaceholder, render_markdown};

// WASM-only components (use gloo_net, reqwasm)
#[cfg(target_arch = "wasm32")]
pub mod message_input;
#[cfg(target_arch = "wasm32")]
pub mod chat_messages;
#[cfg(target_arch = "wasm32")]
pub mod search_bar;

pub use chat_input::ChatInput;
#[cfg(target_arch = "wasm32")]
pub use chat_messages::ChatMessages;
#[cfg(target_arch = "wasm32")]
pub use search_bar::SearchBar;
pub use theme_toggle::ThemeToggle;
pub use theme_selector::{ThemeSelector, SettingsButton};
pub use ai_tool_selector::{AiToolLanding, slug_to_ai_tool, ai_tool_to_slug, ai_tool_display_name, ai_tool_icon};
pub use delete_button::{DeleteButton, InlineDeleteButton};
pub use common::{LoadingText, SessionsLoading, ConversationLoading, ErrorMessage, ProjectCard, EmptyState};
pub use image_upload::{ImageGallery, ImageLightbox, ImagePreviewGrid, ImageUploadButton};
pub use cli_selector::{CliProviderOption, CliSelector, CliSelectorCompact, CliSelectorWithStatus};
