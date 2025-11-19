// Custom Dioxus hooks
pub mod api;
pub mod use_stream_parser;
pub mod use_permissions;
pub mod use_abort_controller;
pub mod use_chat_state;
pub mod use_theme;
pub mod use_image_upload;

// API hooks temporarily disabled due to lifetime complexity
pub use use_stream_parser::use_stream_parser;
pub use use_permissions::{use_permissions, UsePermissionsReturn, PermissionRequest, PlanModeRequest};
pub use use_abort_controller::{use_abort_controller, UseAbortControllerReturn};
pub use use_chat_state::{use_chat_state, ChatState};
pub use use_theme::{use_theme, Theme, save_theme, save_default_theme};
pub use use_image_upload::{use_image_upload, ImageUploadState, upload_file_to_server};
