//! Chat input component with image drag & drop and paste support

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use js_sys::eval as js_eval;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use crate::shared::hooks::ImageUploadState;

#[cfg(target_arch = "wasm32")]
use crate::shared::hooks::{ImageUploadState, upload_file_to_server};
use crate::app::components::ImagePreviewGrid;

/// Setup the JavaScript bridge for file handling (WASM only)
#[cfg(target_arch = "wasm32")]
fn setup_image_handlers(state: ImageUploadState) {
    use wasm_bindgen::JsCast;

    web_sys::console::log_1(&"[WASM] Setting up image handlers".into());

    // Create callback for JS to call when files are dropped/pasted
    let callback = Closure::wrap(Box::new(move |files: js_sys::Array| {
        web_sys::console::log_1(&format!("[WASM] Callback invoked! files.length = {}", files.length()).into());

        for i in 0..files.length() {
            web_sys::console::log_1(&format!("[WASM] Processing file index {}", i).into());

            let file_value = files.get(i);
            web_sys::console::log_1(&format!("[WASM] Got file_value at index {}", i).into());

            match file_value.dyn_into::<web_sys::File>() {
                Ok(file) => {
                    let filename = file.name();
                    web_sys::console::log_1(&format!("[WASM] Got File object: {}", filename).into());

                    let mut state_clone = state.clone();
                    // Use spawn_local instead of Dioxus spawn (no runtime context needed)
                    wasm_bindgen_futures::spawn_local(async move {
                        web_sys::console::log_1(&format!("[WASM] Starting upload for: {}", filename).into());
                        match upload_file_to_server(file, &mut state_clone).await {
                            Ok(upload) => {
                                web_sys::console::log_1(&format!("[WASM] Upload success: {}", upload.filename).into());
                            }
                            Err(e) => {
                                web_sys::console::error_1(&format!("[WASM] Upload failed: {}", e).into());
                            }
                        }
                    });
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("[WASM] dyn_into failed: {:?}", e).into());
                }
            }
        }
    }) as Box<dyn FnMut(js_sys::Array)>);

    // Register callback with window
    if let Some(window) = web_sys::window() {
        let _ = js_sys::Reflect::set(
            &window,
            &JsValue::from_str("__onFilesDropped"),
            callback.as_ref(),
        );
    }
    callback.forget(); // Keep closure alive

    // Setup handlers on DOM elements (with small delay for DOM to be ready)
    let _ = js_eval(r#"
        setTimeout(function() {
            if (window.__setupImageDropZone) {
                window.__setupImageDropZone('chat-input-container');
            }
            if (window.__setupImagePasteHandler) {
                window.__setupImagePasteHandler('chat-input');
            }
            if (window.__setupImageFileInput) {
                window.__setupImageFileInput('image-upload-input');
            }
        }, 100);
    "#);
}

#[component]
pub fn ChatInput(
    input: Signal<String>,
    is_loading: Signal<bool>,
    upload_state: ImageUploadState,
    on_submit: EventHandler<()>,
    on_abort: EventHandler<()>,
) -> Element {
    let mut is_composing = use_signal(|| false);

    // Setup image handlers on mount (WASM only, run once)
    #[cfg(target_arch = "wasm32")]
    {
        let state = upload_state.clone();
        use_effect(move || {
            setup_image_handlers(state.clone());
        });
    }

    // Auto-focus effect
    use_effect(move || {
        if !*is_loading.read() {
            #[cfg(target_arch = "wasm32")]
            {
                let script = r#"
                    setTimeout(() => {
                        const textarea = document.getElementById('chat-input');
                        if (textarea) {
                            textarea.focus();
                        }
                    }, 100);
                "#;
                let _ = js_eval(script);
            }
        }
    });

    // Handle keypress (Enter to send, Shift+Enter for newline)
    let upload_state_for_keypress = upload_state.clone();
    let handle_keypress = move |evt: Event<KeyboardData>| {
        if evt.key() == Key::Enter
            && !evt.modifiers().contains(Modifiers::SHIFT)
            && !*is_composing.read()
        {
            evt.prevent_default();
            let input_value = input();
            let has_images = upload_state_for_keypress.has_images();
            // Allow sending if text is not empty OR if there are images
            if (!input_value.trim().is_empty() || has_images) && !*is_loading.read() {
                on_submit.call(());
            }
        }
    };

    // Handle ESC key for abort
    let handle_keydown = move |evt: Event<KeyboardData>| {
        if evt.key() == Key::Escape && *is_loading.read() {
            on_abort.call(());
        }
    };

    let placeholder = if *is_loading.read() {
        "Processing..."
    } else {
        "Type your message... (Enter to send, Shift+Enter for new line)"
    };

    let has_images = upload_state.has_images();
    let has_content = !input().trim().is_empty() || has_images;

    rsx! {
        div {
            id: "chat-input-container",
            class: "chat-input",

            // Textarea (full width, top)
            textarea {
                id: "chat-input",
                class: "chat-input__textarea",
                value: "{input}",
                placeholder: "{placeholder}",
                disabled: *is_loading.read(),
                rows: "1",
                oninput: move |evt| {
                    input.set(evt.value());
                },
                onkeypress: handle_keypress,
                onkeydown: handle_keydown,
                oncompositionstart: move |_| is_composing.set(true),
                oncompositionend: move |_| is_composing.set(false),
            }

            // Actions row: send button right only
            div { class: "chat-input__actions-row",
                // Left side empty (removed upload button)
                div { class: "chat-input__left-actions" }

                div { class: "chat-input__right-actions",
                    if *is_loading.read() {
                        button {
                            class: "btn btn--abort",
                            onclick: move |_| on_abort.call(()),
                            title: "Stop (ESC)",
                            span { class: "btn__icon", "⏹️" }
                        }
                    }

                    button {
                        class: "btn btn--send btn--icon-only",
                        disabled: !has_content || *is_loading.read(),
                        onclick: move |_evt| {
                            if has_content && !*is_loading.read() {
                                on_submit.call(());
                            }
                        },
                        if *is_loading.read() {
                            span { class: "btn__spinner", "..." }
                        } else {
                            span { class: "btn__icon", "➤" }
                        }
                    }
                }
            }

            // Image preview grid BELOW (like Claude web) - only when has images
            if has_images {
                ImagePreviewGrid {
                    upload_state: upload_state.clone(),
                }
            }
        }
    }
}
