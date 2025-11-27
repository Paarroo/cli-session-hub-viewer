use crate::app::components::claude_viewer::{ConversationView, ProjectsList};
#[cfg(target_arch = "wasm32")]
use crate::app::components::claude_viewer::ImageUpload;
#[cfg(target_arch = "wasm32")]
use crate::app::components::SearchBar;
#[cfg(target_arch = "wasm32")]
use crate::app::pages::ChatPage;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
use crate::app::components::{ThemeToggle, ThemeSelector, SettingsButton, AiToolLanding, slug_to_ai_tool, ai_tool_display_name, ai_tool_icon, ai_tool_to_slug, SessionsLoading};
use crate::server_fns::{get_sessions_summaries, delete_session, get_projects, get_project, SessionSummaryResponse};
use crate::domain::models::AiTool;
use chrono::{DateTime, Utc, Duration, Datelike};

use dioxus::prelude::*;
use dioxus::document;

// Stub components for server-side rendering
#[cfg(not(target_arch = "wasm32"))]
#[component]
fn SearchBar() -> Element {
    rsx! {
        div { class: "search-placeholder",
            style: "padding: 1rem; background: var(--card); border: 1px solid var(--border); border-radius: 8px; margin-bottom: 1rem; color: var(--muted-foreground);",
            "🔍 Search (requires client-side JavaScript)"
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[component]
fn ImageUpload() -> Element {
    rsx! {
        div { class: "image-upload-placeholder",
            style: "padding: 2rem; background: var(--card); border: 1px solid var(--border); border-radius: 8px; text-align: center; color: var(--muted-foreground);",
            "👁️ Image Upload (requires client-side JavaScript)"
        }
    }
}

// Stub ChatPage for server-side rendering
#[cfg(not(target_arch = "wasm32"))]
#[component]
fn ChatPage(
    project_name: String,
    #[props(default)] initial_messages: Vec<crate::domain::models::Message>,
    #[props(default)] initial_session_id: Option<String>,
) -> Element {
    let _ = (initial_messages, initial_session_id); // Suppress unused warnings
    rsx! {
        div { class: "chat-page-placeholder",
            style: "padding: 2rem; text-align: center;",
            div { class: "loading-spinner", "💬" }
            p { "Loading chat interface..." }
        }
    }
}

// Helper function to format relative time (Claude.ai /recents style)
fn format_relative_time(timestamp: &DateTime<Utc>, now: &DateTime<Utc>) -> String {
    let diff = *now - *timestamp;
    let date = timestamp.date_naive();
    let today = now.date_naive();
    let yesterday = today - Duration::days(1);

    if diff.num_minutes() < 1 {
        "à l'instant".to_string()
    } else if diff.num_minutes() < 60 {
        format!("{}min", diff.num_minutes())
    } else if diff.num_hours() < 24 && date == today {
        format!("{}h", diff.num_hours())
    } else if date == yesterday {
        "Hier".to_string()
    } else if diff.num_days() < 7 {
        let day_names = ["Dim", "Lun", "Mar", "Mer", "Jeu", "Ven", "Sam"];
        let weekday = timestamp.weekday().num_days_from_sunday() as usize;
        day_names[weekday].to_string()
    } else {
        let month_names = ["jan.", "fév.", "mars", "avr.", "mai", "juin",
                          "juil.", "août", "sept.", "oct.", "nov.", "déc."];
        let month = timestamp.month0() as usize;
        format!("{} {}", timestamp.day(), month_names[month])
    }
}

// Helper function to group sessions by time period
fn group_sessions_for_display(sessions: &[SessionSummaryResponse], now: &DateTime<Utc>) -> Vec<(String, Vec<SessionSummaryResponse>)> {
    let today = now.date_naive();
    let yesterday = today - Duration::days(1);
    let week_ago = today - Duration::days(7);
    let month_ago = today - Duration::days(30);

    let mut today_sessions = Vec::new();
    let mut yesterday_sessions = Vec::new();
    let mut this_week_sessions = Vec::new();
    let mut this_month_sessions = Vec::new();
    let mut older_sessions = Vec::new();

    // Sort sessions by date, most recent first
    let mut sorted_sessions: Vec<_> = sessions.to_vec();
    sorted_sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    for session in sorted_sessions {
        let session_date = chrono::DateTime::parse_from_rfc3339(&session.updated_at)
            .map(|dt| dt.with_timezone(&Utc).date_naive())
            .unwrap_or(today);

        if session_date == today {
            today_sessions.push(session);
        } else if session_date == yesterday {
            yesterday_sessions.push(session);
        } else if session_date > week_ago {
            this_week_sessions.push(session);
        } else if session_date > month_ago {
            this_month_sessions.push(session);
        } else {
            older_sessions.push(session);
        }
    }

    let mut groups = Vec::new();
    if !today_sessions.is_empty() {
        groups.push(("Aujourd'hui".to_string(), today_sessions));
    }
    if !yesterday_sessions.is_empty() {
        groups.push(("Hier".to_string(), yesterday_sessions));
    }
    if !this_week_sessions.is_empty() {
        groups.push(("Cette semaine".to_string(), this_week_sessions));
    }
    if !this_month_sessions.is_empty() {
        groups.push(("Ce mois".to_string(), this_month_sessions));
    }
    if !older_sessions.is_empty() {
        groups.push(("Plus ancien".to_string(), older_sessions));
    }

    groups
}

// SessionItem component for Project page (Claude.ai /recents style)
#[component]
fn SessionItem(
    session: SessionSummaryResponse,
    tool: String,
    project_name: String,
    encoded_name: String,
    /// Whether selection mode is active
    #[props(default = false)]
    selection_mode: bool,
    /// Whether this session is selected
    #[props(default = false)]
    is_selected: bool,
    /// Callback when checkbox is toggled
    on_toggle_select: Option<EventHandler<String>>,
    /// Callback when session is deleted (legacy mode only)
    on_deleted: Option<EventHandler<String>>,
) -> Element {
    let mut show_confirm = use_signal(|| false);
    let mut is_deleting = use_signal(|| false);

    let now = chrono::Utc::now();

    // Debug: log the raw timestamp value
    #[cfg(debug_assertions)]
    tracing::debug!("Session {} updated_at raw: '{}'", session.session_id, session.updated_at);

    let updated_at = chrono::DateTime::parse_from_rfc3339(&session.updated_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to parse timestamp '{}': {}", session.updated_at, e);
            now
        });
    let relative_time = format_relative_time(&updated_at, &now);

    // Title: first 60 chars of preview or session ID
    let title = if session.preview.is_empty() {
        format!("Session {}", &session.session_id[..8.min(session.session_id.len())])
    } else if session.preview.len() > 60 {
        format!("{}...", session.preview.chars().take(60).collect::<String>())
    } else {
        session.preview.clone()
    };

    let session_id = session.session_id.clone();
    let session_id_for_toggle = session.session_id.clone();
    let session_id_for_delete = session.session_id.clone();
    let encoded_name_for_delete = encoded_name.clone();

    let item_class = if is_selected {
        "c-session-item c-session-item--selected"
    } else {
        "c-session-item"
    };

    rsx! {
        div { class: "{item_class}",
            // Checkbox in selection mode
            if selection_mode {
                label {
                    class: "c-session-item__checkbox-wrapper",
                    onclick: move |evt| {
                        evt.stop_propagation();
                    },
                    input {
                        r#type: "checkbox",
                        class: "c-session-item__checkbox",
                        checked: is_selected,
                        onchange: move |_| {
                            if let Some(handler) = &on_toggle_select {
                                handler.call(session_id_for_toggle.clone());
                            }
                        },
                    }
                }
            }

            // Normal content (always rendered)
            Link {
                class: "c-session-item__link",
                to: Route::Session {
                    tool: tool.clone(),
                    project_name: project_name.clone(),
                    session_id: session_id.clone()
                },

                // Icon (hidden in selection mode)
                if !selection_mode {
                    div { class: "c-session-item__icon", "💬" }
                }

                // Content
                div { class: "c-session-item__content",
                    div { class: "c-session-item__title", "{title}" }
                    div { class: "c-session-item__meta",
                        span { "{session.message_count} messages" }
                    }
                }

                // Relative time
                div { class: "c-session-item__time", "{relative_time}" }
            }

            // Delete button (only in normal mode, not in selection mode)
            if !selection_mode {
                button {
                    class: "c-session-item__delete",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        evt.prevent_default();
                        show_confirm.set(true);
                    },
                    "🗑️"
                }
            }

            // Confirmation overlay (shown on top when confirming)
            if *show_confirm.read() && !selection_mode {
                div { class: "c-session-item__confirm-overlay",
                    span { class: "c-session-item__confirm-text", "Supprimer ?" }
                    button {
                        class: "c-session-item__confirm-btn c-session-item__confirm-btn--danger",
                        disabled: *is_deleting.read(),
                        onclick: move |evt| {
                            evt.stop_propagation();
                            let session_id = session_id_for_delete.clone();
                            let encoded = encoded_name_for_delete.clone();
                            let on_deleted = on_deleted;
                            is_deleting.set(true);
                            spawn(async move {
                                match delete_session(encoded, session_id.clone()).await {
                                    Ok(_) => {
                                        if let Some(handler) = on_deleted {
                                            handler.call(session_id);
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to delete session: {:?}", e);
                                        is_deleting.set(false);
                                        show_confirm.set(false);
                                    }
                                }
                            });
                        },
                        if *is_deleting.read() { "..." } else { "Oui" }
                    }
                    button {
                        class: "c-session-item__confirm-btn c-session-item__confirm-btn--cancel",
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

#[derive(Clone, Routable, Debug, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Layout)]
    // Landing page - AI tool selection
    #[route("/")]
    Home {},

    // Tool-specific routes with :tool prefix
    #[route("/:tool")]
    ToolHome { tool: String },
    #[route("/:tool/projects/:project_name")]
    Project { tool: String, project_name: String },
    #[route("/:tool/projects/:project_name/sessions/:session_id")]
    Session {
        tool: String,
        project_name: String,
        session_id: String
    },
    #[route("/:tool/projects/:project_name/chat")]
    Chat { tool: String, project_name: String },

    // Legacy routes (kept for backward compatibility)
    #[route("/vision")]
    Vision {},
}

#[component]
pub fn App() -> Element {
    // Debug logging pour vérifier l'initialisation
    use_effect(|| {
        tracing::info!("Dioxus App initialized successfully");
    });

    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn Layout() -> Element {
    // Use asset!() macro to ensure CSS is bundled and served correctly
    const BUNDLE_CSS: Asset = asset!("/assets/bundle.css");

    rsx! {
        document::Link {
            rel: "stylesheet",
            href: BUNDLE_CSS
        },
        // Load WASM bundle for client-side hydration
        document::Script {
            src: "/wasm/cli-session-hub-viewer.js",
            r#type: "module"
        },
        div { class: "c-layout",
            // Global navbar spanning full width
            AppNavbar {}

            // Body: sidebar + main content
            div { class: "c-layout__body",
                // Left sidebar with project navigation
                AppSidebar {}

                // Main content area
                main { class: "c-layout__main",
                    Outlet::<Route> {}
                }
            }
        }
    }
}

/// Global navbar with logo and theme toggle
#[component]
fn AppNavbar() -> Element {
    rsx! {
        nav { class: "c-navbar",
            // Left: Logo
            Link {
                to: Route::Home {},
                class: "c-navbar__logo",
                "📜 CLI Session Hub"
            }

            // Right: Theme toggle only
            div { class: "c-navbar__actions",
                ThemeToggle {}
            }
        }
    }
}

/// Sidebar component with tool navigation and project list
#[component]
fn AppSidebar() -> Element {
    // Search filter state
    let mut search_query = use_signal(String::new);
    // Settings panel state
    let mut settings_open = use_signal(|| false);

    // Fetch all projects for sidebar navigation
    let projects_resource = use_server_future(move || async move {
        get_projects(None, None).await
    })?;

    // Group and filter projects by AI tool
    let grouped_projects = match &*projects_resource.read() {
        Some(Ok(projects)) => {
            let query = search_query.read().to_lowercase();
            let mut claude_projects = Vec::new();
            let mut opencode_projects = Vec::new();
            let mut gemini_projects = Vec::new();

            for project in projects {
                // Filter by search query
                if !query.is_empty() && !project.name.to_lowercase().contains(&query) {
                    continue;
                }

                match project.ai_tool {
                    AiTool::ClaudeCode => claude_projects.push(project.clone()),
                    AiTool::OpenCode => opencode_projects.push(project.clone()),
                    AiTool::Gemini => gemini_projects.push(project.clone()),
                }
            }

            Some((claude_projects, opencode_projects, gemini_projects))
        }
        _ => None,
    };

    rsx! {
        aside { class: "c-sidebar",
            // Main tool navigation
            nav { class: "c-sidebar__nav-main",
                Link {
                    to: Route::Home {},
                    class: "c-sidebar__nav-item",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        web_sys::console::log_1(&"[Sidebar] Accueil link clicked!".into());
                    },
                    span { class: "c-sidebar__nav-icon", "🏠" }
                    span { class: "c-sidebar__nav-text", "Accueil" }
                }
            }

            // Search input (functional)
            div { class: "c-sidebar__search",
                input {
                    r#type: "text",
                    class: "c-sidebar__search-input",
                    placeholder: "🔍 Rechercher...",
                    value: search_query(),
                    oninput: move |evt| search_query.set(evt.value())
                }
            }

            // Navigation with project groups
            nav { class: "c-sidebar__nav",
                if let Some((claude, opencode, gemini)) = grouped_projects {
                    // Claude Code section
                    if !claude.is_empty() {
                        SidebarToolSection {
                            tool: AiTool::ClaudeCode,
                            projects: claude,
                        }
                    }

                    // OpenCode section
                    if !opencode.is_empty() {
                        SidebarToolSection {
                            tool: AiTool::OpenCode,
                            projects: opencode,
                        }
                    }

                    // Gemini section
                    if !gemini.is_empty() {
                        SidebarToolSection {
                            tool: AiTool::Gemini,
                            projects: gemini,
                        }
                    }
                } else {
                    // Loading state
                    div { class: "c-sidebar__loading",
                        "Chargement..."
                    }
                }
            }

            // Footer with settings button
            div { class: "c-sidebar__footer",
                SettingsButton {
                    on_click: move |_| settings_open.set(true),
                }
            }

            // Theme selector panel
            ThemeSelector { is_open: settings_open }
        }
    }
}

/// Sidebar section for a specific AI tool
#[component]
fn SidebarToolSection(tool: AiTool, projects: Vec<crate::server_fns::ProjectResponse>) -> Element {
    let tool_name = ai_tool_display_name(&tool);
    let tool_icon = ai_tool_icon(&tool);
    let tool_slug = ai_tool_to_slug(&tool);

    rsx! {
        div { class: "c-sidebar__section",
            // Section header (clickable - navigates to tool home)
            Link {
                to: Route::ToolHome { tool: tool_slug.to_string() },
                class: "c-sidebar__section-header",
                "data-tool": "{tool_slug}",
                "{tool_icon} {tool_name}"
                span { class: "c-sidebar__count", "{projects.len()}" }
            }

            // Project items
            for project in projects {
                Link {
                    to: Route::Project {
                        tool: tool_slug.to_string(),
                        project_name: project.encoded_name.clone()
                    },
                    class: "c-sidebar__item",
                    "data-tool": "{tool_slug}",
                    span { class: "c-sidebar__item-icon", "📁" }
                    span { class: "c-sidebar__item-text", "{project.name}" }
                    span { class: "c-sidebar__count", "{project.session_count}" }
                }
            }
        }
    }
}

#[component]
fn Home() -> Element {
    rsx! {
        AiToolLanding {}
    }
}

#[component]
fn ToolHome(tool: String) -> Element {
    // Validate tool slug
    let ai_tool = slug_to_ai_tool(&tool);
    let tool_name = ai_tool.as_ref().map(|t| ai_tool_display_name(t)).unwrap_or("Unknown");
    let tool_icon = ai_tool.as_ref().map(|t| ai_tool_icon(t)).unwrap_or("❓");

    // If invalid tool, show error
    if ai_tool.is_none() {
        return rsx! {
            div { class: "error-page",
                h1 { "Unknown AI Tool" }
                p { "The tool \"{tool}\" is not recognized." }
                Link { to: Route::Home {}, class: "breadcrumb-link", "← Back to Home" }
            }
        };
    }

    rsx! {
        div { class: "home-page",
            div { class: "home-container",
                // Page header with tool info
                header { class: "page-header",
                    div { class: "page-header__breadcrumb",
                        Link {
                            to: Route::Home {},
                            class: "breadcrumb-link",
                            "← All Tools"
                        }
                    }
                    h1 { class: "page-title",
                        "{tool_icon} {tool_name} Projects"
                    }
                    p { class: "page-description",
                        "Browse and manage your {tool_name} sessions"
                    }
                }

                // Search bar
                SearchBar {}

                // Projects grid - with Suspense for SSR compatibility
                SuspenseBoundary {
                    fallback: |_| rsx! {
                        div { class: "loading-projects", "Chargement des projets..." }
                    },
                    ProjectsListFiltered { tool: tool.clone() }
                }
            }
        }
    }
}

/// ProjectsList filtered by AI tool
#[component]
fn ProjectsListFiltered(tool: String) -> Element {
    let _ai_tool = slug_to_ai_tool(&tool);

    // Reuse ProjectsList with server-side filtering
    rsx! {
        ProjectsList { tool_filter: Some(tool) }
    }
}

#[component]
fn Project(tool: String, project_name: String) -> Element {
    // Decode project name for display
    let display_name = project_name
        .split('-')
        .filter(|s| !s.is_empty())
        .next_back()
        .unwrap_or(&project_name)
        .to_string();

    // Clone project_name for use in closure and rsx BEFORE any moves
    let project_name_for_delete = project_name.clone();

    // Track deleted session IDs to filter them out (reactive)
    let mut deleted_ids: Signal<Vec<String>> = use_signal(Vec::new);

    // Selection mode state
    let mut selection_mode = use_signal(|| false);
    let mut selected_ids: Signal<std::collections::HashSet<String>> = use_signal(std::collections::HashSet::new);
    let mut is_batch_deleting = use_signal(|| false);

    // REACTIVE: Store current props in signals
    // use_signal initializes with first render value, then we update when props change
    let mut current_tool = use_signal(|| tool.clone());
    let mut current_project = use_signal(|| project_name.clone());

    // Detect prop changes by comparing signal values with current props
    let props_changed = current_tool() != tool || current_project() != project_name;

    if props_changed {
        tracing::info!("Route changed: {}:{} -> {}:{}", current_tool(), current_project(), tool, project_name);
        // Update signals with new prop values
        current_tool.set(tool.clone());
        current_project.set(project_name.clone());
        // Reset component state
        deleted_ids.write().clear();
        selected_ids.write().clear();
        selection_mode.set(false);
    }

    // Use use_resource reading from signals - will re-fetch when signals change
    let sessions_resource = use_resource(move || {
        let tool_slug = current_tool();
        let proj_name = current_project();
        async move {
            tracing::info!("Fetching sessions for: {} (tool: {})", proj_name, tool_slug);
            get_sessions_summaries(proj_name, Some(tool_slug)).await
        }
    });

    // Handler for session deletion - add to deleted list
    let on_session_deleted = move |deleted_session_id: String| {
        deleted_ids.write().push(deleted_session_id);
    };

    // Toggle selection for a single session
    let on_toggle_select = move |session_id: String| {
        let mut ids = selected_ids.write();
        if ids.contains(&session_id) {
            ids.remove(&session_id);
        } else {
            ids.insert(session_id);
        }
    };

    // Enter selection mode
    let enter_selection_mode = move |_| {
        selection_mode.set(true);
        selected_ids.write().clear();
    };

    // Exit selection mode
    let exit_selection_mode = move |_| {
        selection_mode.set(false);
        selected_ids.write().clear();
    };

    rsx! {
        div { class: "project-page",
            // Back link at top
            Link {
                to: Route::ToolHome { tool: current_tool() },
                class: "breadcrumb-link",
                "← Back to Projects"
            }

            // Header with title and selection toggle
            div { class: "project-page__header",
                h1 { "Project: {display_name}" }

                // Show "Sélectionner" link when not in selection mode
                if !selection_mode() {
                    button {
                        class: "c-sessions__select-btn",
                        onclick: enter_selection_mode,
                        "Sélectionner"
                    }
                }
            }

            match &*sessions_resource.read() {
                Some(Ok(sessions)) => {
                    // Filter out deleted sessions
                    let deleted = deleted_ids.read();
                    let filtered_sessions: Vec<_> = sessions
                        .iter()
                        .filter(|s| !deleted.contains(&s.session_id))
                        .cloned()
                        .collect();

                    // Get all session IDs for select all
                    let all_session_ids: Vec<String> = filtered_sessions.iter().map(|s| s.session_id.clone()).collect();
                    let total_count = all_session_ids.len();
                    let selected_count = selected_ids.read().len();
                    let all_selected = selected_count == total_count && total_count > 0;

                    // Group sessions by time period
                    let now = chrono::Utc::now();
                    let grouped = group_sessions_for_display(&filtered_sessions, &now);

                    // Clone for closures
                    let encoded_name_for_batch = project_name_for_delete.clone();

                    rsx! {
                        // Selection action bar (visible in selection mode)
                        if selection_mode() {
                            div { class: "c-selection-bar",
                                // Select all checkbox
                                label { class: "c-selection-bar__select-all",
                                    input {
                                        r#type: "checkbox",
                                        checked: all_selected,
                                        onchange: move |_| {
                                            let mut ids = selected_ids.write();
                                            if all_selected {
                                                ids.clear();
                                            } else {
                                                for id in &all_session_ids {
                                                    ids.insert(id.clone());
                                                }
                                            }
                                        },
                                    }
                                    span { "Tout" }
                                }

                                // Counter
                                span { class: "c-selection-bar__count",
                                    if selected_count <= 1 {
                                        "{selected_count} sélectionné"
                                    } else {
                                        "{selected_count} sélectionnés"
                                    }
                                }

                                // Action buttons
                                div { class: "c-selection-bar__actions",
                                    // Delete button
                                    button {
                                        class: "c-selection-bar__action c-selection-bar__action--delete",
                                        disabled: selected_count == 0 || is_batch_deleting(),
                                        title: "Supprimer",
                                        onclick: move |_| {
                                            let ids_to_delete: Vec<String> = selected_ids.read().iter().cloned().collect();
                                            let encoded_name = encoded_name_for_batch.clone();

                                            spawn(async move {
                                                is_batch_deleting.set(true);

                                                for session_id in ids_to_delete {
                                                    match delete_session(encoded_name.clone(), session_id.clone()).await {
                                                        Ok(_) => {
                                                            deleted_ids.write().push(session_id.clone());
                                                            selected_ids.write().remove(&session_id);
                                                        }
                                                        Err(e) => {
                                                            tracing::error!("Failed to delete session {}: {:?}", session_id, e);
                                                        }
                                                    }
                                                }

                                                is_batch_deleting.set(false);
                                                // Exit selection mode after batch delete
                                                selection_mode.set(false);
                                            });
                                        },
                                        if is_batch_deleting() {
                                            "⏳"
                                        } else {
                                            "🗑️"
                                        }
                                    }
                                }

                                // Close button
                                button {
                                    class: "c-selection-bar__close",
                                    onclick: exit_selection_mode,
                                    title: "Annuler",
                                    "✕"
                                }
                            }
                        }

                        div { class: "c-sessions",
                            if grouped.is_empty() {
                                div { class: "c-sessions__empty",
                                    div { class: "c-sessions__empty-icon", "💬" }
                                    div { class: "c-sessions__empty-title", "Aucune session" }
                                    div { class: "c-sessions__empty-description", "Les conversations apparaîtront ici." }
                                }
                            } else {
                                for (group_name, group_sessions) in grouped.iter() {
                                    if !group_sessions.is_empty() {
                                        div { class: "c-sessions__group-header", "{group_name}" }
                                        for session in group_sessions {
                                            SessionItem {
                                                session: session.clone(),
                                                tool: current_tool(),
                                                project_name: current_project(),
                                                encoded_name: project_name_for_delete.clone(),
                                                selection_mode: selection_mode(),
                                                is_selected: selected_ids.read().contains(&session.session_id),
                                                on_toggle_select: on_toggle_select,
                                                on_deleted: on_session_deleted,
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Err(e)) => rsx! {
                    p { "Error loading sessions: {e:?}" }
                },
                None => rsx! {
                    SessionsLoading {}
                }
            }
        }
    }
}



#[component]
fn Session(tool: String, project_name: String, session_id: String) -> Element {
    let tool_clone = tool.clone();
    let project_name_clone = project_name.clone();

    rsx! {
        div { class: "c-session-page",
            // Simple header with back link
            div { class: "c-session-page__header",
                Link {
                    to: Route::Project { tool: tool_clone.clone(), project_name: project_name_clone.clone() },
                    class: "c-session-page__back-link",
                    "← Retour aux sessions"
                }
            }

            // Main conversation area - full width
            main { class: "c-session-page__content",
                ConversationView {
                    key: "{session_id}",
                    project_name: project_name.clone(),
                    session_id: session_id.clone()
                }
            }
        }
    }
}

#[component]
fn Vision() -> Element {
    rsx! {
        div { class: "main-content",
            div { class: "container",
                header { class: "page-header",
                    h1 { class: "page-title",
                        "👁️ "
                        "Vision API"
                    }
                    p { class: "page-description",
                        "Upload and analyze images with Claude"
                    }
                }

                main {
                    ImageUpload {}
                }
            }
        }
    }
}

#[component]
fn Chat(tool: String, project_name: String) -> Element {
    let tool_clone = tool.clone();
    let project_name_clone = project_name.clone();
    let ai_tool = slug_to_ai_tool(&tool);
    let tool_name = ai_tool.as_ref().map(|t| ai_tool_display_name(t)).unwrap_or("AI");

    // State for manually entered working directory (for new projects)
    let mut manual_working_dir = use_signal(|| None::<String>);
    let mut path_input = use_signal(String::new);
    let mut path_error = use_signal(|| None::<String>);

    // Fetch the project to get its real filesystem path
    let project_resource = use_resource(move || {
        let encoded_name = project_name_clone.clone();
        let tool_slug = tool_clone.clone();
        async move {
            get_project(encoded_name, Some(tool_slug)).await
        }
    });

    // Fetch active session on mount to link with existing Claude Code session
    let active_session_id = use_signal(|| None::<String>);

    #[cfg(target_arch = "wasm32")]
    {
        let project_name_for_session = project_name.clone();
        let mut active_session_id = active_session_id.clone();

        use_effect(move || {
            let project_name = project_name_for_session.clone();
            wasm_bindgen_futures::spawn_local(async move {
                // Fetch active session from API
                let url = format!("/api/projects/{}/active-session", project_name);

                if let Some(window) = web_sys::window() {
                    if let Ok(resp) = wasm_bindgen_futures::JsFuture::from(
                        window.fetch_with_str(&url)
                    ).await {
                        if let Ok(response) = resp.dyn_into::<web_sys::Response>() {
                            if response.ok() {
                                if let Ok(json) = wasm_bindgen_futures::JsFuture::from(
                                    response.json().unwrap()
                                ).await {
                                    if let Some(session_id) = js_sys::Reflect::get(&json, &wasm_bindgen::JsValue::from_str("session_id"))
                                        .ok()
                                        .and_then(|v| v.as_string())
                                    {
                                        tracing::info!("Linking to existing session: {}", session_id);
                                        active_session_id.set(Some(session_id));
                                    }
                                }
                            }
                        }
                    }
                }
            });
        });
    }

    let session_id_value = (*active_session_id.read()).clone();
    let tool_for_link = tool.clone();

    // Extract project data from resource (clone to avoid borrow issues)
    let project_data = project_resource.read().clone();

    match project_data {
        Some(Ok(Some(project))) => {
            let working_dir = project.path.clone();
            let display_name = project.name.clone();

            rsx! {
                div { class: "main-content chat-container",
                    div { class: "container",
                        header { class: "page-header",
                            div { class: "page-header__row",
                                div {
                                    h1 { class: "page-title",
                                        "💬 Chat with {tool_name}"
                                    }
                                    p { class: "page-description",
                                        "Project: {display_name}"
                                    }
                                    p { class: "page-description",
                                        style: "font-size: 0.75rem; color: var(--muted-foreground);",
                                        "📁 {working_dir}"
                                    }
                                    if session_id_value.is_some() {
                                        p { class: "page-description session-linked",
                                            style: "font-size: 0.75rem; color: var(--primary);",
                                            "🔗 Linked to active session"
                                        }
                                    }
                                }
                                Link {
                                    to: Route::Project { tool: tool_for_link.clone(), project_name: project_name.clone() },
                                    class: "breadcrumb-link",
                                    "← Back to Sessions"
                                }
                            }
                        }

                        main { class: "chat-main",
                            ChatPage {
                                project_name: working_dir,
                                initial_session_id: session_id_value,
                            }
                        }
                    }
                }
            }
        }
        Some(Ok(None)) => {
            // Project not found - check if user has manually entered a path
            if let Some(working_dir) = manual_working_dir.read().clone() {
                // User has selected a directory, show the chat
                rsx! {
                    div { class: "main-content chat-container",
                        div { class: "container",
                            header { class: "page-header",
                                div { class: "page-header__row",
                                    div {
                                        h1 { class: "page-title",
                                            "💬 Chat with {tool_name}"
                                        }
                                        p { class: "page-description",
                                            "New session"
                                        }
                                        p { class: "page-description",
                                            style: "font-size: 0.75rem; color: var(--muted-foreground);",
                                            "📁 {working_dir}"
                                        }
                                    }
                                    Link {
                                        to: Route::ToolHome { tool: tool_for_link.clone() },
                                        class: "breadcrumb-link",
                                        "← Back to Projects"
                                    }
                                }
                            }

                            main { class: "chat-main",
                                ChatPage {
                                    project_name: working_dir,
                                    initial_session_id: session_id_value.clone(),
                                }
                            }
                        }
                    }
                }
            } else {
                // Show directory selector
                let current_input = path_input.read().clone();
                let current_error = path_error.read().clone();

                rsx! {
                    div { class: "main-content",
                        div { class: "container",
                            div { class: "directory-selector",
                                style: "max-width: 600px; margin: 2rem auto; padding: 2rem; background: var(--card); border-radius: 12px; border: 1px solid var(--border);",

                                h2 { style: "margin-bottom: 1rem; color: var(--foreground);",
                                    "📁 Select Working Directory"
                                }
                                p { style: "margin-bottom: 1.5rem; color: var(--muted-foreground);",
                                    "Enter the path to the directory where you want to work with {tool_name}. "
                                    "This is where the AI will have access to read and modify files."
                                }

                                div { style: "display: flex; flex-direction: column; gap: 1rem;",
                                    input {
                                        r#type: "text",
                                        placeholder: "e.g., /Users/you/projects/my-app",
                                        value: "{current_input}",
                                        style: "width: 100%; padding: 0.75rem 1rem; border: 1px solid var(--border); border-radius: 8px; background: var(--background); color: var(--foreground); font-size: 1rem;",
                                        oninput: move |e| {
                                            path_input.set(e.value().clone());
                                            path_error.set(None);
                                        }
                                    }

                                    if let Some(error) = current_error {
                                        p { style: "color: var(--destructive); font-size: 0.875rem;",
                                            "{error}"
                                        }
                                    }

                                    div { style: "display: flex; gap: 1rem; justify-content: flex-end;",
                                        Link {
                                            to: Route::ToolHome { tool: tool_for_link.clone() },
                                            class: "btn btn--secondary",
                                            "Cancel"
                                        }
                                        button {
                                            class: "btn btn--primary",
                                            disabled: current_input.trim().is_empty(),
                                            onclick: move |_| {
                                                let path = path_input.read().trim().to_string();
                                                if path.is_empty() {
                                                    path_error.set(Some("Please enter a path".to_string()));
                                                } else if !path.starts_with('/') && !path.starts_with('~') {
                                                    path_error.set(Some("Path must be absolute (start with / or ~)".to_string()));
                                                } else {
                                                    // Expand ~ to home directory representation
                                                    let expanded_path = if path.starts_with('~') {
                                                        // On web we can't expand ~, so we'll let the server handle it
                                                        path.clone()
                                                    } else {
                                                        path.clone()
                                                    };
                                                    manual_working_dir.set(Some(expanded_path));
                                                }
                                            },
                                            "Start Chat →"
                                        }
                                    }
                                }

                                // Quick suggestions
                                div { style: "margin-top: 1.5rem; padding-top: 1.5rem; border-top: 1px solid var(--border);",
                                    p { style: "font-size: 0.875rem; color: var(--muted-foreground); margin-bottom: 0.75rem;",
                                        "Quick suggestions:"
                                    }
                                    div { style: "display: flex; flex-wrap: wrap; gap: 0.5rem;",
                                        button {
                                            class: "btn btn--secondary",
                                            style: "font-size: 0.875rem; padding: 0.5rem 1rem;",
                                            onclick: move |_| path_input.set("~/Desktop".to_string()),
                                            "~/Desktop"
                                        }
                                        button {
                                            class: "btn btn--secondary",
                                            style: "font-size: 0.875rem; padding: 0.5rem 1rem;",
                                            onclick: move |_| path_input.set("~/Documents".to_string()),
                                            "~/Documents"
                                        }
                                        button {
                                            class: "btn btn--secondary",
                                            style: "font-size: 0.875rem; padding: 0.5rem 1rem;",
                                            onclick: move |_| path_input.set("~/projects".to_string()),
                                            "~/projects"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Some(Err(e)) => {
            let error_msg = e.to_string();
            rsx! {
                div { class: "main-content",
                    div { class: "container",
                        div { class: "error-state",
                            h2 { "Error loading project" }
                            p { "{error_msg}" }
                            Link {
                                to: Route::ToolHome { tool: tool_for_link.clone() },
                                class: "breadcrumb-link",
                                "← Back to Projects"
                            }
                        }
                    }
                }
            }
        }
        None => {
            // Loading state
            rsx! {
                div { class: "main-content chat-container",
                    div { class: "container",
                        header { class: "page-header",
                            div { class: "page-header__row",
                                div {
                                    h1 { class: "page-title",
                                        "💬 Chat with {tool_name}"
                                    }
                                    p { class: "page-description",
                                        "Loading project..."
                                    }
                                }
                                Link {
                                    to: Route::Project { tool: tool_for_link.clone(), project_name: project_name.clone() },
                                    class: "breadcrumb-link",
                                    "← Back to Sessions"
                                }
                            }
                        }
                        main { class: "chat-main",
                            div { class: "loading-spinner",
                                "⏳ Loading..."
                            }
                        }
                    }
                }
            }
        }
    }
}
