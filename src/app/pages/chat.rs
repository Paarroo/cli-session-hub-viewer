use crate::app::components::{ChatInput, ChatMessages};
use crate::domain::models::{ChatRequest, Message};
use crate::shared::hooks::{use_chat_state, use_image_upload};
use crate::shared::utils::process_stream_line;
use chrono::Utc;
use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Request, RequestInit, RequestMode, Response};

#[component]
pub fn ChatPage(
    project_name: String,
    #[props(default)] initial_messages: Vec<Message>,
    #[props(default)] initial_session_id: Option<String>,
) -> Element {
    // Log for debugging
    tracing::info!(
        "ChatPage rendering with {} initial messages, session: {:?}",
        initial_messages.len(),
        initial_session_id
    );

    // Create local signals with initial values
    let mut messages = use_signal(|| initial_messages.clone());
    let input = use_signal(|| String::new());
    let is_loading = use_signal(|| false);
    let upload_state = use_image_upload();
    let mut current_session_id = use_signal(|| initial_session_id.clone());
    let current_request_id = use_signal(|| None::<String>);
    let current_assistant_message = use_signal(|| None::<Message>);

    // Sync props to signals when they change (for when parent reloads data)
    let initial_messages_len = initial_messages.len();
    let initial_session_clone = initial_session_id.clone();
    use_effect(move || {
        // Only update if we have new data and current is empty
        if initial_messages_len > 0 && messages.read().is_empty() {
            tracing::info!("Syncing {} messages from props", initial_messages_len);
            messages.set(initial_messages.clone());
        }
        if initial_session_clone.is_some() && current_session_id.read().is_none() {
            current_session_id.set(initial_session_clone.clone());
        }
    });

    // Build chat state from local signals
    let mut chat_state = crate::shared::hooks::ChatState {
        messages,
        input,
        is_loading,
        current_session_id,
        current_request_id,
        current_assistant_message,
    };

    // Send message handler
    let send_message = {
        let mut chat_state = chat_state.clone();
        let project_name = project_name.clone();
        let mut upload_state_for_send = upload_state.clone();

        move |message_content: String| {
            let mut chat_state = chat_state.clone();
            let project_name = project_name.clone();
            let mut upload_state_clone = upload_state_for_send.clone();

            spawn_local(async move {
                // Get images from upload state BEFORE clearing
                let pending_count = upload_state_clone.pending_images.read().len();
                let uploaded_count = upload_state_clone.uploaded_images.read().len();

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("[WASM] 📸 Send message - pending: {}, uploaded: {}", pending_count, uploaded_count).into());

                let image_paths = upload_state_clone.get_image_paths();
                let uploaded_images: Vec<crate::domain::models::ImageAttachment> =
                    upload_state_clone.uploaded_images.read().clone();

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("[WASM] 📸 Image paths to send: {:?}", image_paths).into());

                // 1. Add user message to chat WITH images
                chat_state.add_message(Message::User {
                    content: message_content.clone(),
                    timestamp: Utc::now(),
                    images: uploaded_images,
                    metadata: None,
                });

                // 2. Generate request ID
                let request_id = chat_state.generate_request_id();

                // 3. Clear input, images and start loading
                chat_state.clear_input();
                upload_state_clone.clear();  // Clear images after capturing them
                chat_state.start_request();

                // 4. Build request WITH images
                let mut chat_request = ChatRequest::new(message_content.clone(), request_id.clone())
                    .with_working_directory(project_name.clone());

                // Add images if any
                if !image_paths.is_empty() {
                    tracing::info!("Adding {} images to request", image_paths.len());
                    chat_request = chat_request.with_images(image_paths);
                }

                // Add session_id if exists
                let chat_request = if let Some(session_id) = (*chat_state.current_session_id.read()).clone() {
                    chat_request.with_session_id(session_id)
                } else {
                    chat_request
                };

                let request_body = match serde_json::to_string(&chat_request) {
                    Ok(body) => body,
                    Err(e) => {
                        tracing::error!("Failed to serialize request: {}", e);
                        chat_state.add_message(Message::System {
                            content: format!("Error: Failed to serialize request - {}", e),
                            timestamp: Utc::now(),
                            level: Some(crate::domain::models::LogLevel::Error),
                            metadata: None,
                        });
                        chat_state.reset_request_state();
                        return;
                    }
                };

                // 5. Send request with streaming using web_sys fetch API
                let window = match web_sys::window() {
                    Some(w) => w,
                    None => {
                        tracing::error!("No window object available");
                        chat_state.reset_request_state();
                        return;
                    }
                };

                // Build fetch request
                let mut opts = RequestInit::new();
                opts.method("POST");
                opts.mode(RequestMode::SameOrigin);
                opts.body(Some(&JsValue::from_str(&request_body)));

                let request = match Request::new_with_str_and_init("/api/chat/native", &opts) {
                    Ok(req) => req,
                    Err(e) => {
                        tracing::error!("Failed to create request: {:?}", e);
                        chat_state.add_message(Message::System {
                            content: format!("Error: Failed to create request"),
                            timestamp: Utc::now(),
                            level: Some(crate::domain::models::LogLevel::Error),
                            metadata: None,
                        });
                        chat_state.reset_request_state();
                        return;
                    }
                };

                // Set content-type header
                if let Err(e) = request.headers().set("Content-Type", "application/json") {
                    tracing::error!("Failed to set header: {:?}", e);
                }

                // Fetch with streaming
                let resp_promise = window.fetch_with_request(&request);
                let resp_value = match JsFuture::from(resp_promise).await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!("Fetch failed: {:?}", e);
                        chat_state.add_message(Message::System {
                            content: format!("Error: Network request failed"),
                            timestamp: Utc::now(),
                            level: Some(crate::domain::models::LogLevel::Error),
                            metadata: None,
                        });
                        chat_state.reset_request_state();
                        return;
                    }
                };

                let response: Response = resp_value.dyn_into().unwrap();

                // Check response status
                if !response.ok() {
                    let status = response.status();
                    tracing::error!("Server error: {}", status);
                    chat_state.add_message(Message::System {
                        content: format!("Server error: {}", status),
                        timestamp: Utc::now(),
                        level: Some(crate::domain::models::LogLevel::Error),
                        metadata: None,
                    });
                    chat_state.reset_request_state();
                    return;
                }

                // 6. Stream response body (NDJSON - newline delimited)
                let body = match response.body() {
                    Some(b) => b,
                    None => {
                        tracing::error!("No response body");
                        chat_state.reset_request_state();
                        return;
                    }
                };

                let reader = body.get_reader().dyn_into::<web_sys::ReadableStreamDefaultReader>().unwrap();
                let mut buffer = String::new();

                // Read stream chunks
                loop {
                    let result = match JsFuture::from(reader.read()).await {
                        Ok(r) => r,
                        Err(e) => {
                            tracing::error!("Error reading stream: {:?}", e);
                            break;
                        }
                    };

                    let done = js_sys::Reflect::get(&result, &JsValue::from_str("done"))
                        .unwrap_or(JsValue::TRUE)
                        .as_bool()
                        .unwrap_or(true);

                    if done {
                        break;
                    }

                    let value = js_sys::Reflect::get(&result, &JsValue::from_str("value"))
                        .ok()
                        .and_then(|v| v.dyn_into::<js_sys::Uint8Array>().ok());

                    if let Some(chunk) = value {
                        let bytes = chunk.to_vec();
                        if let Ok(text) = String::from_utf8(bytes) {
                            buffer.push_str(&text);

                            // Process complete NDJSON lines
                            while let Some(newline_pos) = buffer.find('\n') {
                                let line = buffer[..newline_pos].to_string();
                                buffer = buffer[newline_pos + 1..].to_string();

                                if line.trim().is_empty() {
                                    continue;
                                }

                                // Process each line with stream parser (real-time update)
                                process_stream_line(
                                    &line,
                                    chat_state.messages,
                                    chat_state.current_assistant_message,
                                    chat_state.current_session_id,
                                    chat_state.is_loading,
                                );
                            }
                        }
                    }
                }

                // Process any remaining buffer content
                if !buffer.trim().is_empty() {
                    process_stream_line(
                        &buffer,
                        chat_state.messages,
                        chat_state.current_assistant_message,
                        chat_state.current_session_id,
                        chat_state.is_loading,
                    );
                }
            });
        }
    };

    // Abort handler
    let abort_request = {
        let mut chat_state = chat_state.clone();

        move || {
            if let Some(request_id) = (*chat_state.current_request_id.read()).clone() {
                let request_id = request_id.clone();
                let mut chat_state = chat_state.clone();

                spawn_local(async move {
                    let url = format!("/api/abort/{}", request_id);

                    if let Some(window) = web_sys::window() {
                        let mut opts = RequestInit::new();
                        opts.method("POST");
                        opts.mode(RequestMode::SameOrigin);

                        if let Ok(request) = Request::new_with_str_and_init(&url, &opts) {
                            let resp_promise = window.fetch_with_request(&request);
                            match JsFuture::from(resp_promise).await {
                                Ok(v) => {
                                    if let Ok(response) = v.dyn_into::<Response>() {
                                        if response.ok() {
                                            tracing::info!("Request aborted successfully");
                                        } else {
                                            tracing::error!("Abort failed: {}", response.status());
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Abort request error: {:?}", e);
                                }
                            }
                        }
                    }

                    chat_state.reset_request_state();
                });
            }
        }
    };

    rsx! {
        div { class: "chat-page",
            ChatMessages {
                messages: chat_state.messages,
            }
            ChatInput {
                input: chat_state.input,
                is_loading: chat_state.is_loading,
                upload_state: upload_state.clone(),
                on_submit: move |_| {
                    let input_value = (*chat_state.input.read()).clone();
                    if !input_value.trim().is_empty() && !*chat_state.is_loading.read() {
                        send_message(input_value);
                    }
                },
                on_abort: move |_| abort_request(),
            }
        }
    }
}
