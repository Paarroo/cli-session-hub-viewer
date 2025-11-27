use dioxus::prelude::*;
use keyboard_types::Modifiers;
use crate::domain::models::{AiTool, PermissionMode};
use crate::domain::models::Message;

#[cfg(any(target_arch = "wasm32", target_arch = "wasm32"))]
use crate::shared::hooks::use_stream_parser;

#[cfg(any(target_arch = "wasm32", target_arch = "wasm32"))]
use wasm_bindgen::prelude::*;

#[cfg(any(target_arch = "wasm32", target_arch = "wasm32"))]
use wasm_bindgen::JsCast;

#[cfg(any(target_arch = "wasm32", target_arch = "wasm32"))]
use web_sys::{EventSource, MessageEvent};

#[cfg(any(target_arch = "wasm32", target_arch = "wasm32"))]
use reqwasm::http::Request;

#[component]
pub fn MessageInput(
    project_path: String,
    session_id: Option<String>,
    ai_tool: AiTool,
    streaming_messages: Signal<Vec<Message>>,
    is_streaming: Signal<bool>,
) -> Element {
    let mut message = use_signal(|| String::new());
    let mut permission_mode = use_signal(|| PermissionMode::Read);
    let stream_parser = use_stream_parser();

    // Clone values before creating closures
    let project_path_keypress = project_path.clone();
    let session_id_keypress = session_id.clone();
    let ai_tool_keypress = ai_tool.clone();

    let handle_keypress = move |evt: Event<KeyboardData>| {
        if evt.key() == Key::Enter && !evt.modifiers().contains(Modifiers::SHIFT) {
            evt.prevent_default();

            if message().trim().is_empty() || *is_streaming.read() {
                return;
            }

            let msg_content = message();
            let proj_path = project_path_keypress.clone();
            let sess_id = session_id_keypress.clone();
            let ai_str = match ai_tool_keypress.clone() {
                AiTool::ClaudeCode => "claude",
                AiTool::OpenCode => "opencode",
                AiTool::Gemini => "gemini",
            };
            let perm_mode = permission_mode();

            message.set(String::new());
            is_streaming.set(true);

            spawn(async move {
                let url = format!(
                    "/api/sessions/{}/{}/stream?tool={}&permission={}",
                    proj_path,
                    sess_id.unwrap_or_default(),
                    ai_str,
                    perm_mode as i32
                );

                match Request::post(&url)
                    .body(&msg_content)
                    .send()
                    .await
                {
                    Ok(_) => {
                        is_streaming.set(false);
                    }
                    Err(e) => {
                        tracing::error!("Stream error: {}", e);
                        is_streaming.set(false);
                    }
                }
            });
        }
    };

    rsx! {
        div { class: "message-input-container",
            style: "
                border-top: 1px solid var(--border);
                padding: 1rem;
                background: var(--background);
            ",

            // Permission mode selector
            div { style: "margin-bottom: 0.75rem;",
                label {
                    style: "
                        display: block;
                        font-size: 0.875rem;
                        font-weight: 500;
                        margin-bottom: 0.25rem;
                        color: var(--foreground);
                    ",
                    "Permission Mode:"
                }
                select {
                    value: "{permission_mode().as_str()}",
                    onchange: move |evt| {
                        let mode = match evt.value().as_str() {
                            "read" => PermissionMode::Read,
                            "write" => PermissionMode::Write,
                            "execute" => PermissionMode::Execute,
                            _ => PermissionMode::Read,
                        };
                        permission_mode.set(mode);
                    },
                    style: "
                        padding: 0.375rem 0.75rem;
                        border: 1px solid var(--border);
                        border-radius: 0.375rem;
                        background: var(--background);
                        color: var(--foreground);
                        font-size: 0.875rem;
                    ",
                    option { value: "normal", "Normal (Ask for permissions)" }
                    option { value: "plan", "Plan (Read-only)" }
                    option { value: "bypasspermissions", "Bypass (Auto-approve all)" }
                    option { value: "accept_edits", "Accept Edits (Auto-approve file changes)" }
                    option { value: "dont_ask", "Don't Ask (Never ask)" }
                }
            }

            // Message input area
            div { style: "display: flex; gap: 0.75rem; align-items: flex-end;",
                textarea {
                    placeholder: "Type your message to {ai_tool.as_str()}...",
                    value: "{message}",
                    oninput: move |evt| message.set(evt.value()),
                    onkeypress: handle_keypress,
                    disabled: *is_streaming.read(),
                    rows: "3",
                    style: "
                        flex: 1;
                        padding: 0.75rem;
                        border: 1px solid var(--border);
                        border-radius: 0.5rem;
                        background: var(--background);
                        color: var(--foreground);
                        font-size: 0.875rem;
                        resize: vertical;
                        min-height: 3rem;
                        max-height: 8rem;
                    "
                }

                button {
                    onclick: {
                        let project_path_btn = project_path.clone();
                        let session_id_btn = session_id.clone();
                        let ai_tool_btn = ai_tool.clone();
                        move |_| {
                            if message().trim().is_empty() || *is_streaming.read() {
                                return;
                            }

                            let msg_content = message();
                            let proj_path = project_path_btn.clone();
                            let sess_id = session_id_btn.clone();
                            let ai_str = match ai_tool_btn.clone() {
                                AiTool::ClaudeCode => "claude",
                                AiTool::OpenCode => "opencode",
                                AiTool::Gemini => "gemini",
                            };
                            let perm_mode = permission_mode();

                            message.set(String::new());
                            is_streaming.set(true);

                            spawn(async move {
                                let url = format!(
                                    "/api/sessions/{}/{}/stream?tool={}&permission={}",
                                    proj_path,
                                    sess_id.unwrap_or_default(),
                                    ai_str,
                                    perm_mode as i32
                                );

                                match Request::post(&url)
                                    .body(&msg_content)
                                    .send()
                                    .await
                                {
                                    Ok(_) => {
                                        is_streaming.set(false);
                                    }
                                    Err(e) => {
                                        tracing::error!("Stream error: {}", e);
                                        is_streaming.set(false);
                                    }
                                }
                            });
                        }
                    },
                    disabled: *is_streaming.read() || message().trim().is_empty(),
                    style: "
                        padding: 0.75rem 1.5rem;
                        background: var(--primary);
                        color: var(--primary-foreground);
                        border: none;
                        border-radius: 0.5rem;
                        font-size: 0.875rem;
                        font-weight: 500;
                        cursor: pointer;
                        transition: background 150ms ease-in-out;
                        display: flex;
                        align-items: center;
                        gap: 0.5rem;
                    ",
                    if *is_streaming.read() {
                        "Streaming..."
                    } else {
                        "Send"
                    }
                }
            }

            // Help text
            div { style: "
                margin-top: 0.5rem;
                font-size: 0.75rem;
                color: var(--muted-foreground);
            ",
                "Press Enter to send, Shift+Enter for new line"
            }
        }
    }
}

impl PermissionMode {
    fn as_str(&self) -> &'static str {
        match self {
            PermissionMode::Read => "read",
            PermissionMode::Write => "write",
            PermissionMode::Execute => "execute",
        }
    }
}

impl AiTool {
    fn as_str(&self) -> &'static str {
        match self {
            AiTool::ClaudeCode => "Claude Code",
            AiTool::OpenCode => "OpenCode",
            AiTool::Gemini => "Gemini",
        }
    }
}
