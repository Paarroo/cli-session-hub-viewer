use crate::domain::models::Message;
use dioxus::prelude::*;
use uuid::Uuid;

/// Chat state management hook
#[derive(Clone)]
pub struct ChatState {
    pub messages: Signal<Vec<Message>>,
    pub input: Signal<String>,
    pub is_loading: Signal<bool>,
    pub current_session_id: Signal<Option<String>>,
    pub current_request_id: Signal<Option<String>>,
    pub current_assistant_message: Signal<Option<Message>>,
}

impl ChatState {
    /// Add a message to the chat
    pub fn add_message(&mut self, message: Message) {
        self.messages.write().push(message);
    }

    /// Clear the input field
    pub fn clear_input(&mut self) {
        self.input.set(String::new());
    }

    /// Generate a new request ID
    pub fn generate_request_id(&mut self) -> String {
        let request_id = Uuid::new_v4().to_string();
        self.current_request_id.set(Some(request_id.clone()));
        request_id
    }

    /// Reset request state (after completion or abort)
    pub fn reset_request_state(&mut self) {
        self.is_loading.set(false);
        self.current_request_id.set(None);
        self.current_assistant_message.set(None);
    }

    /// Start a new request
    pub fn start_request(&mut self) {
        self.is_loading.set(true);
    }
}

/// Hook to manage chat state
pub fn use_chat_state() -> ChatState {
    let messages = use_signal(Vec::<Message>::new);
    let input = use_signal(String::new);
    let is_loading = use_signal(|| false);
    let current_session_id = use_signal(|| None::<String>);
    let current_request_id = use_signal(|| None::<String>);
    let current_assistant_message = use_signal(|| None::<Message>);

    ChatState {
        messages,
        input,
        is_loading,
        current_session_id,
        current_request_id,
        current_assistant_message,
    }
}
