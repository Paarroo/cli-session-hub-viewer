//! AI Tool Selector Component
//! Landing page component for selecting between Claude Code, OpenCode, and Gemini CLI

use dioxus::prelude::*;
use crate::domain::models::AiTool;
use crate::server_fns::get_projects;

/// Tool card data for display
#[derive(Clone)]
struct ToolCardData {
    #[allow(dead_code)]
    tool: AiTool,
    name: &'static str,
    icon: &'static str,
    description: &'static str,
    route_slug: &'static str,
}

const TOOLS: &[ToolCardData] = &[
    ToolCardData {
        tool: AiTool::ClaudeCode,
        name: "Claude Code",
        icon: "🤖",
        description: "Anthropic's CLI for Claude AI assistance",
        route_slug: "claude",
    },
    ToolCardData {
        tool: AiTool::OpenCode,
        name: "OpenCode",
        icon: "⚡",
        description: "Open-source AI coding assistant",
        route_slug: "opencode",
    },
    ToolCardData {
        tool: AiTool::Gemini,
        name: "Gemini CLI",
        icon: "🧠",
        description: "Google's Gemini AI in the terminal",
        route_slug: "gemini",
    },
];

/// Landing page with AI tool selection cards - only shows tools with sessions
#[component]
pub fn AiToolLanding() -> Element {
    // Fetch projects to know which tools have sessions
    let projects_resource = use_server_future(move || async move {
        get_projects(None, None).await
    })?;

    // Determine which tools have projects
    let tools_with_projects: Vec<&'static str> = match &*projects_resource.read() {
        Some(Ok(projects)) => {
            let mut tools = Vec::new();
            for project in projects {
                let slug = match project.ai_tool {
                    AiTool::ClaudeCode => "claude",
                    AiTool::OpenCode => "opencode",
                    AiTool::Gemini => "gemini",
                };
                if !tools.contains(&slug) {
                    tools.push(slug);
                }
            }
            tools
        }
        _ => vec![], // Loading or error - show nothing yet
    };

    // Filter tools to only show those with projects
    let visible_tools: Vec<_> = TOOLS
        .iter()
        .filter(|t| tools_with_projects.contains(&t.route_slug))
        .collect();

    rsx! {
        div { class: "c-tool-landing",
            // Header
            header { class: "c-tool-landing__header",
                h1 { class: "c-tool-landing__title",
                    "CLI Session Hub"
                }
                p { class: "c-tool-landing__subtitle",
                    "View and manage your AI coding assistant sessions"
                }
            }

            // Tool cards grid - only tools with sessions
            div { class: "c-tool-landing__grid",
                if visible_tools.is_empty() {
                    // Show loading or empty state
                    match &*projects_resource.read() {
                        None => rsx! {
                            p { class: "c-tool-landing__loading", "Chargement..." }
                        },
                        Some(Err(_)) => rsx! {
                            p { class: "c-tool-landing__error", "Erreur de chargement" }
                        },
                        Some(Ok(_)) => rsx! {
                            p { class: "c-tool-landing__empty", "Aucune session trouvée" }
                        },
                    }
                } else {
                    for tool_data in visible_tools {
                        ToolCard {
                            icon: tool_data.icon,
                            name: tool_data.name,
                            description: tool_data.description,
                            route_slug: tool_data.route_slug,
                        }
                    }
                }
            }
        }
    }
}

/// Individual tool selection card
#[component]
fn ToolCard(
    icon: &'static str,
    name: &'static str,
    description: &'static str,
    route_slug: &'static str,
) -> Element {
    // Default project name for new sessions
    let default_project = format!("{}-default-project", route_slug);

    rsx! {
        // Entire card redirects to new chat session
        Link {
            to: crate::app::pages::claude_routes::Route::Chat {
                tool: route_slug.to_string(),
                project_name: default_project
            },
            class: "c-tool-card",
            div { class: "c-tool-card__icon", "{icon}" }
            h2 { class: "c-tool-card__name", "{name}" }
            p { class: "c-tool-card__description", "{description}" }
            span { class: "c-tool-card__arrow", "→" }
        }
    }
}

/// Convert route slug to AiTool enum
pub fn slug_to_ai_tool(slug: &str) -> Option<AiTool> {
    match slug {
        "claude" => Some(AiTool::ClaudeCode),
        "opencode" => Some(AiTool::OpenCode),
        "gemini" => Some(AiTool::Gemini),
        _ => None,
    }
}

/// Convert AiTool enum to route slug
pub fn ai_tool_to_slug(tool: &AiTool) -> &'static str {
    match tool {
        AiTool::ClaudeCode => "claude",
        AiTool::OpenCode => "opencode",
        AiTool::Gemini => "gemini",
    }
}

/// Get display name for AI tool
pub fn ai_tool_display_name(tool: &AiTool) -> &'static str {
    match tool {
        AiTool::ClaudeCode => "Claude Code",
        AiTool::OpenCode => "OpenCode",
        AiTool::Gemini => "Gemini CLI",
    }
}

/// Get icon for AI tool
pub fn ai_tool_icon(tool: &AiTool) -> &'static str {
    match tool {
        AiTool::ClaudeCode => "🤖",
        AiTool::OpenCode => "⚡",
        AiTool::Gemini => "🧠",
    }
}
