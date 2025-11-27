use crate::domain::models::{LogLevel, Message, TodoItem};
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use js_sys::eval as js_eval;

#[component]
pub fn ChatMessages(messages: Signal<Vec<Message>>) -> Element {
    // Auto-scroll to bottom effect
    use_effect(move || {
        let messages_read = messages.read();
        if !messages_read.is_empty() {
            // Scroll to bottom using JavaScript
            #[cfg(target_arch = "wasm32")]
            {
                let script = r#"
                    setTimeout(() => {
                        const messagesEnd = document.getElementById('messages-end');
                        if (messagesEnd) {
                            messagesEnd.scrollIntoView({ behavior: 'smooth' });
                        }
                    }, 100);
                "#;
                let _ = js_eval(script);
            }
        }
    });

    rsx! {
        div { class: "chat-messages",
            if messages.read().is_empty() {
                EmptyState {}
            } else {
                for message in messages.read().iter() {
                    MessageItem { message: message.clone() }
                }
                div { id: "messages-end" }
            }
        }
    }
}

#[component]
fn EmptyState() -> Element {
    rsx! {
        div { class: "empty-state",
            div { class: "empty-state__icon", "💬" }
            h2 { class: "empty-state__title", "Start a conversation" }
            p { class: "empty-state__description",
                "Send a message to begin interacting with Claude"
            }
        }
    }
}

#[component]
fn MessageItem(message: Message) -> Element {
    match message {
        Message::User { content, timestamp, .. } => {
            let time_str = timestamp.format("%H:%M").to_string();
            rsx! {
                div { class: "message message--user",
                    div { class: "message__content", dangerous_inner_html: "{content}" }
                    span { class: "message__timestamp", "{time_str}" }
                }
            }
        }

        Message::Assistant { content, timestamp, .. } => {
            let time_str = timestamp.format("%H:%M").to_string();
            // TODO: Render markdown properly
            let html = render_markdown(&content);
            rsx! {
                div { class: "message message--assistant",
                    div { class: "message__content", dangerous_inner_html: "{html}" }
                    span { class: "message__timestamp", "{time_str}" }
                }
            }
        }

        Message::Thinking { content, timestamp, .. } => {
            let time_str = timestamp.format("%H:%M:%S").to_string();
            rsx! {
                div { class: "message message--thinking",
                    div { class: "message__header",
                        span { class: "message__icon", "💭" }
                        span { class: "message__label", "Thinking" }
                        span { class: "message__timestamp", "{time_str}" }
                    }
                    div { class: "message__content message__content--thinking", "{content}" }
                }
            }
        }

        Message::Tool { name, input, output, timestamp, .. } => {
            let time_str = timestamp.format("%H:%M:%S").to_string();
            let input_json = serde_json::to_string_pretty(&input).unwrap_or_default();
            rsx! {
                div { class: "message message--tool",
                    div { class: "tool__header",
                        span { class: "tool__icon", "🛠️" }
                        span { class: "tool__name", "{name}" }
                        span { class: "tool__timestamp", "{time_str}" }
                    }
                    details { class: "tool__details",
                        summary { "Input" }
                        pre { class: "tool__json", "{input_json}" }
                    }
                    if let Some(out) = output {
                        details { class: "tool__details",
                            summary { "Output" }
                            pre { class: "tool__output", "{out}" }
                        }
                    }
                }
            }
        }

        Message::System { content, level, timestamp, .. } => {
            let time_str = timestamp.format("%H:%M:%S").to_string();
            let (icon, level_class) = match level {
                Some(LogLevel::Error) => ("❌", "system--error"),
                Some(LogLevel::Warn) => ("⚠️", "system--warn"),
                Some(LogLevel::Info) => ("ℹ️", "system--info"),
                Some(LogLevel::Debug) => ("🐛", "system--debug"),
                None => ("⚙️", "system--default"),
            };

            rsx! {
                div { class: "message message--system message--{level_class}",
                    span { class: "system__icon", "{icon}" }
                    span { class: "system__content", "{content}" }
                    span { class: "system__timestamp", "{time_str}" }
                }
            }
        }

        Message::Plan { content, timestamp, .. } => {
            let time_str = timestamp.format("%H:%M:%S").to_string();
            let html = render_markdown(&content);
            rsx! {
                div { class: "message message--plan",
                    div { class: "plan__header",
                        span { class: "plan__icon", "📋" }
                        span { class: "plan__label", "Plan Proposed" }
                        span { class: "plan__timestamp", "{time_str}" }
                    }
                    div { class: "plan__content", dangerous_inner_html: "{html}" }
                    div { class: "plan__actions",
                        button { class: "btn btn--secondary", "Keep Planning" }
                        button { class: "btn btn--primary", "Accept with Edits" }
                        button { class: "btn btn--primary", "Accept Default" }
                    }
                }
            }
        }

        Message::Todo { items, timestamp, .. } => {
            let time_str = timestamp.format("%H:%M:%S").to_string();
            rsx! {
                div { class: "message message--todo",
                    div { class: "todo__header",
                        span { class: "todo__icon", "✅" }
                        span { class: "todo__label", "TODO List" }
                        span { class: "todo__timestamp", "{time_str}" }
                    }
                    ul { class: "todo__list",
                        for item in items {
                            TodoItemComponent { item: item.clone() }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TodoItemComponent(item: TodoItem) -> Element {
    let status_icon = match item.status.as_str() {
        "completed" => "✅",
        "in_progress" => "🔄",
        _ => "⏳",
    };

    let status_class = match item.status.as_str() {
        "completed" => "todo-item--completed",
        "in_progress" => "todo-item--in-progress",
        _ => "todo-item--pending",
    };

    rsx! {
        li { class: "todo-item {status_class}",
            span { class: "todo-item__icon", "{status_icon}" }
            span { class: "todo-item__content",
                if item.status == "in_progress" {
                    "{item.active_form}"
                } else {
                    "{item.content}"
                }
            }
        }
    }
}

/// Simple markdown renderer (basic implementation)
fn render_markdown(content: &str) -> String {
    // TODO: Use proper markdown library like pulldown-cmark
    // For now, simple replacements
    let mut html = content
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\n\n", "</p><p>")
        .replace("\n", "<br>");

    // Wrap in paragraph if not already
    if !html.starts_with("<p>") {
        html = format!("<p>{}</p>", html);
    }

    // Bold **text**
    html = html.replace("**", "<strong>").replace("**", "</strong>");

    // Italic *text*
    html = html.replace("*", "<em>").replace("*", "</em>");

    // Code `text`
    html = html.replace("`", "<code>").replace("`", "</code>");

    html
}
