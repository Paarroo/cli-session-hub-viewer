//! Message rendering component
//!
//! Displays individual messages in the conversation view

use dioxus::prelude::*;
use pulldown_cmark::{html, Options, Parser};
use crate::app::components::ImageGallery;

/// Helper function to render Markdown to HTML
pub fn render_markdown(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

/// Renders a single message in the conversation
#[component]
pub fn MessageItem(message: crate::domain::models::Message) -> Element {
    use crate::domain::models::Message;

    match message {
        Message::User {
            content, timestamp, images, ..
        } => rsx! {
            li { class: "c-chat-message c-chat-message--user animate-fade-in",
                div { class: "c-chat-bubble c-chat-bubble--user",
                    // Display images if any
                    if !images.is_empty() {
                        div { class: "c-chat-bubble__images",
                            ImageGallery {
                                images: images.clone(),
                                lightbox_enabled: true,
                            }
                        }
                    }
                    div { class: "c-chat-bubble__content",
                        div { class: "u-whitespace-pre-wrap", {content.clone()} }
                    }
                    span { class: "c-chat-bubble__timestamp",
                        {timestamp.format("%H:%M").to_string()}
                    }
                }
            }
        },
        Message::Assistant {
            content, timestamp, images, ..
        } => {
            let html_content = render_markdown(&content);
            rsx! {
                li { class: "c-chat-message c-chat-message--assistant animate-fade-in",
                    div { class: "c-chat-bubble c-chat-bubble--assistant",
                        div { class: "c-chat-bubble__content",
                            div {
                                class: "c-prose c-prose--sm",
                                dangerous_inner_html: "{html_content}"
                            }
                        }
                        // Display images if any (generated or referenced)
                        if !images.is_empty() {
                            div { class: "c-chat-bubble__images",
                                ImageGallery {
                                    images: images.clone(),
                                    lightbox_enabled: true,
                                }
                            }
                        }
                        span { class: "c-chat-bubble__timestamp",
                            {timestamp.format("%H:%M").to_string()}
                        }
                    }
                }
            }
        }
        Message::Tool {
            name,
            input,
            output,
            timestamp,
            ..
        } => rsx! {
            li { class: "c-conversation-item c-conversation-item--align-start animate-fade-in",
                div { class: "c-conversation-content",
                    div { class: "card tool-message gap-2 py-3 mb-2 rounded-lg",
                        div { class: "py-0 px-4",
                            div { class: "flex items-center justify-between mb-2",
                                span { class: "text-sm font-medium", "🛠️ {name}" }
                                span { class: "text-xs text-muted-foreground",
                                    {timestamp.format("%H:%M:%S").to_string()}
                                }
                            }
                        }
                        div { class: "py-0 px-4",
                            details { class: "mb-2",
                                summary { class: "text-xs font-medium text-muted-foreground cursor-pointer mb-2",
                                    "Input"
                                }
                                div { class: "border rounded p-2 mb-2",
                                    pre { class: "text-xs whitespace-pre-wrap break-all font-mono overflow-x-auto",
                                        {serde_json::to_string_pretty(&input).unwrap_or_default()}
                                    }
                                }
                            }
                            if let Some(o) = output {
                                details {
                                    summary { class: "text-xs font-medium text-muted-foreground cursor-pointer mb-2",
                                        "Output"
                                    }
                                    div { class: "border rounded p-2",
                                        pre { class: "text-xs whitespace-pre-wrap break-all font-mono overflow-x-auto",
                                            {o}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
        Message::System {
            content, timestamp, ..
        } => rsx! {
            li { class: "c-conversation-item c-conversation-item--align-start animate-fade-in",
                div { class: "c-conversation-content",
                    div { class: "card system-message px-3 py-3 mb-3 rounded-lg",
                        div { class: "flex items-center justify-between mb-2",
                            span { class: "text-sm font-medium", "⚙️ System" }
                            span { class: "text-xs text-muted-foreground",
                                {timestamp.format("%H:%M:%S").to_string()}
                            }
                        }
                        div { class: "whitespace-pre-wrap text-sm", {content.clone()} }
                    }
                }
            }
        },
        Message::Thinking {
            content, timestamp, ..
        } => rsx! {
            li { class: "c-conversation-item c-conversation-item--align-start animate-fade-in",
                div { class: "c-conversation-content",
                    div { class: "card thinking-message px-3 py-3 mb-3 rounded-lg",
                        div { class: "flex items-center justify-between mb-2",
                            span { class: "text-sm font-medium", "💭 Thinking" }
                            span { class: "text-xs text-muted-foreground",
                                {timestamp.format("%H:%M:%S").to_string()}
                            }
                        }
                        div { class: "whitespace-pre-wrap text-sm", {content.clone()} }
                    }
                }
            }
        },
        Message::Plan {
            content, timestamp, ..
        } => {
            let html_content = render_markdown(&content);
            rsx! {
                li { class: "c-conversation-item c-conversation-item--align-start animate-fade-in",
                    div { class: "c-conversation-content",
                        div { class: "card plan-message px-3 py-3 mb-3 rounded-lg",
                            div { class: "flex items-center justify-between mb-2",
                                span { class: "text-sm font-medium", "📋 Plan Proposed" }
                                span { class: "text-xs text-muted-foreground",
                                    {timestamp.format("%H:%M:%S").to_string()}
                                }
                            }
                            div {
                                class: "c-prose c-prose--sm",
                                dangerous_inner_html: "{html_content}"
                            }
                        }
                    }
                }
            }
        },
        Message::Todo {
            items, timestamp, ..
        } => rsx! {
            li { class: "c-conversation-item c-conversation-item--align-start animate-fade-in",
                div { class: "c-conversation-content",
                    div { class: "card todo-message px-3 py-3 mb-3 rounded-lg",
                        div { class: "flex items-center justify-between mb-2",
                            span { class: "text-sm font-medium", "✅ TODO List" }
                            span { class: "text-xs text-muted-foreground",
                                {timestamp.format("%H:%M:%S").to_string()}
                            }
                        }
                        ul { class: "space-y-2 mt-2",
                            for item in items {
                                li { class: "flex items-start gap-2",
                                    span { class: "text-sm",
                                        {if item.status == "completed" { "✅" } else if item.status == "in_progress" { "🔄" } else { "⏳" }}
                                    }
                                    span { class: "text-sm",
                                        {if item.status == "in_progress" { item.active_form.clone() } else { item.content.clone() }}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
    }
}

/// Placeholder for message input (read-only in history viewer)
#[component]
pub fn MessageInputPlaceholder() -> Element {
    rsx! {
        div {
            class: "c-message-placeholder",
            "💬 Message input is read-only in history viewer"
        }
    }
}
