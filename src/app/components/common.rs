use dioxus::prelude::*;
use crate::domain::models::ApiProject;
use crate::app::components::ai_tool_to_slug;
use crate::server_fns::delete_project;

// Reusable Loading Component (BEM: c-loading)
// Used across all CLI tools (Claude, OpenCode, Gemini)
#[component]
pub fn LoadingText(message: String) -> Element {
    rsx! {
        div { class: "c-loading",
            div { class: "c-loading__spinner" }
            p { class: "c-loading__text", "{message}" }
        }
    }
}

// Loading variant for sessions list
#[component]
pub fn SessionsLoading() -> Element {
    rsx! {
        div { class: "c-loading c-loading--sessions",
            div { class: "c-loading__spinner" }
            p { class: "c-loading__text", "Chargement des sessions..." }
        }
    }
}

// Loading variant for conversation
#[component]
pub fn ConversationLoading() -> Element {
    rsx! {
        div { class: "c-loading c-loading--conversation",
            div { class: "c-loading__spinner" }
            p { class: "c-loading__text", "Chargement de la conversation..." }
        }
    }
}

// Reusable Error Message Component (BEM: c-error)
#[component]
pub fn ErrorMessage(message: String) -> Element {
    rsx! {
        div { class: "c-error",
            span { class: "c-error__icon", "❌" }
            p { class: "c-error__text", "{message}" }
        }
    }
}

// Reusable Project Card Component
#[component]
pub fn ProjectCard(
    project: ApiProject,
    #[props(default)] on_deleted: Option<EventHandler<String>>,
) -> Element {
    let mut show_confirm = use_signal(|| false);
    let mut is_deleting = use_signal(|| false);

    // Format project name (remove leading dash if present)
    let display_name = project.name.trim_start_matches('-').to_string();

    // Determine AI tool badge
    let (ai_emoji, ai_text) = match project.ai_tool {
        crate::domain::models::AiTool::ClaudeCode => ("🤖", "Claude"),
        crate::domain::models::AiTool::OpenCode => ("⚡", "OpenCode"),
        crate::domain::models::AiTool::Gemini => ("🧠", "Gemini"),
    };

    // Format path for display (show last 2 segments)
    let path_display = project.path
        .split('/')
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .iter()
        .rev()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join("/");

    // Card variant based on AI tool
    let card_class = match project.ai_tool {
        crate::domain::models::AiTool::ClaudeCode => "project-card project-card--claudecode",
        crate::domain::models::AiTool::OpenCode => "project-card project-card--opencode",
        crate::domain::models::AiTool::Gemini => "project-card project-card--gemini",
    };

    let tool_slug = ai_tool_to_slug(&project.ai_tool).to_string();
    let encoded_name_for_delete = project.encoded_name.clone();

    rsx! {
        div { class: "project-card-wrapper",
            Link {
                to: crate::app::pages::claude_routes::Route::Project {
                    tool: tool_slug,
                    project_name: project.encoded_name.clone()
                },
                class: "project-link",
                div {
                    class: "{card_class}",
                    div {
                        class: "project-header",
                        h3 { "📁 {display_name}" }
                        span { class: "ai-badge", "{ai_emoji} {ai_text}" }
                    }
                    p {
                        class: "project-path",
                        title: "{project.path}",
                        "📂 {path_display}"
                    }
                    p { class: "project-sessions", "{project.session_count}" }
                }
            }

            // Delete button
            button {
                class: "project-card__delete",
                onclick: move |evt| {
                    evt.stop_propagation();
                    evt.prevent_default();
                    show_confirm.set(true);
                },
                "🗑️"
            }

            // Confirmation overlay
            if *show_confirm.read() {
                div { class: "project-card__confirm-overlay",
                    span { class: "project-card__confirm-text", "Supprimer ce projet ?" }
                    div { class: "project-card__confirm-actions",
                        button {
                            class: "project-card__confirm-btn project-card__confirm-btn--danger",
                            disabled: *is_deleting.read(),
                            onclick: move |evt| {
                                evt.stop_propagation();
                                let encoded = encoded_name_for_delete.clone();
                                let on_deleted = on_deleted;
                                is_deleting.set(true);
                                spawn(async move {
                                    match delete_project(encoded.clone()).await {
                                        Ok(_) => {
                                            if let Some(handler) = on_deleted {
                                                handler.call(encoded);
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Failed to delete project: {:?}", e);
                                            is_deleting.set(false);
                                            show_confirm.set(false);
                                        }
                                    }
                                });
                            },
                            if *is_deleting.read() { "..." } else { "Oui" }
                        }
                        button {
                            class: "project-card__confirm-btn project-card__confirm-btn--cancel",
                            onclick: move |evt| {
                                evt.stop_propagation();
                                show_confirm.set(false);
                            },
                            "Non"
                        }
                    }
                }
            }
        }
    }
}

// SessionCard is defined in claude_viewer.rs (more complex version with delete functionality)

// Reusable Empty State Component
#[component]
pub fn EmptyState(
    icon: String,
    title: String,
    description: String,
    action_text: Option<String>,
    action_handler: Option<EventHandler>
) -> Element {
    rsx! {
        div {
            class: "empty-state",
            style: "
                text-align: center;
                padding: 3rem 1rem;
                color: #6b7280;
            ",
            div {
                style: "font-size: 3rem; margin-bottom: 1rem;",
                "{icon}"
            }
            h3 {
                style: "margin: 0 0 0.5rem 0; font-size: 1.25rem; font-weight: 600; color: var(--foreground);",
                "{title}"
            }
            p {
                style: "margin: 0 0 1.5rem 0; max-width: 28rem; margin-left: auto; margin-right: auto; color: var(--muted-foreground);",
                "{description}"
            }
            if let (Some(text), Some(handler)) = (action_text, action_handler) {
                button {
                    onclick: move |_| handler.call(()),
                    class: "btn btn--primary",
                    "{text}"
                }
            }
        }
    }
}

// Simplified data loading pattern - use individual components instead