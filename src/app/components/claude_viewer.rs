use dioxus::prelude::*;
use crate::domain::models::{ApiProject, Session, Message};
use crate::server_fns::{get_projects, get_sessions_summaries, delete_session};
use super::common::SessionsLoading;
use super::message_item::MessageItem;
use super::ai_tool_to_slug;
use chrono::{DateTime, Utc, Duration, Datelike};

fn get_group(updated_at: &DateTime<Utc>, now: &DateTime<Utc>) -> String {
    let date = updated_at.date_naive();
    let today = now.date_naive();
    let yesterday = today - Duration::days(1);
    if date == today {
        "Aujourd'hui".to_string()
    } else if date == yesterday {
        "Hier".to_string()
    } else if *updated_at >= *now - Duration::days(7) {
        "7 derniers jours".to_string()
    } else if *updated_at >= *now - Duration::days(30) {
        "30 derniers jours".to_string()
    } else {
        "Plus ancien".to_string()
    }
}

/// Format time as relative string like Claude.ai (e.g., "2h", "Yesterday", "Nov 15")
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
        // Day name for this week
        let day_names = ["Dim", "Lun", "Mar", "Mer", "Jeu", "Ven", "Sam"];
        let weekday = timestamp.weekday().num_days_from_sunday() as usize;
        day_names[weekday].to_string()
    } else {
        // Date format for older: "15 nov."
        let month_names = ["jan.", "fév.", "mars", "avr.", "mai", "juin",
                          "juil.", "août", "sept.", "oct.", "nov.", "déc."];
        let month = timestamp.month0() as usize;
        format!("{} {}", timestamp.day(), month_names[month])
    }
}

fn group_sessions(session_list: Vec<Session>, now: &DateTime<Utc>) -> Vec<(String, Vec<Session>)> {
    let mut groups: Vec<(String, Vec<Session>)> = vec![
        ("Aujourd'hui".to_string(), vec![]),
        ("Hier".to_string(), vec![]),
        ("7 derniers jours".to_string(), vec![]),
        ("30 derniers jours".to_string(), vec![]),
        ("Plus ancien".to_string(), vec![]),
    ];
    for session in session_list {
        let group = get_group(&session.updated_at, now);
        let index = match group.as_str() {
            "Aujourd'hui" => 0,
            "Hier" => 1,
            "7 derniers jours" => 2,
            "30 derniers jours" => 3,
            _ => 4,
        };
        groups[index].1.push(session);
    }
    groups
}

/// Check if a message has no meaningful content to display
fn is_message_empty(message: &Message) -> bool {
    match message {
        Message::User { content, .. } => content.trim().is_empty(),
        Message::Assistant { content, .. } => content.trim().is_empty(),
        Message::System { content, .. } => content.trim().is_empty(),
        Message::Thinking { content, .. } => content.trim().is_empty(),
        Message::Plan { content, .. } => content.trim().is_empty(),
        Message::Tool { output, .. } => {
            // Tool messages are empty only if they have no output
            output.as_ref().map(|o| o.trim().is_empty()).unwrap_or(true)
        }
        Message::Todo { items, .. } => items.is_empty(),
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
use web_sys::{EventSource, HtmlElement};

#[cfg(target_arch = "wasm32")]
use gloo_net::http::Request;

// Import components
#[cfg(target_arch = "wasm32")]
use super::message_input::MessageInput;
use super::common::{LoadingText, ErrorMessage, ProjectCard, EmptyState};

// Projects List Component
#[component]
pub fn ProjectsList(#[props(default)] tool_filter: Option<String>) -> Element {
    // Use server function to load projects - the ? propagates suspense
    let projects_resource = use_server_future(move || {
        let filter = tool_filter.clone();
        async move {
            get_projects(None, filter).await
        }
    })?;

    // Read the result - this will be Some after the future resolves
    let content = match &*projects_resource.read() {
        Some(Ok(project_list)) => {
            let api_projects: Vec<ApiProject> = project_list
                .iter()
                .map(|p| ApiProject {
                    name: p.name.clone(),
                    path: p.path.clone(),
                    session_count: p.session_count,
                    ai_tool: p.ai_tool.clone(),
                    encoded_name: p.encoded_name.clone(),
                })
                .collect();

            if api_projects.is_empty() {
                rsx! {
                    EmptyState {
                        icon: "📁".to_string(),
                        title: "No projects found".to_string(),
                        description: "Create your first project to get started with Claude Code Viewer.".to_string(),
                        action_text: None,
                        action_handler: None,
                    }
                }
            } else {
                rsx! {
                    for project in api_projects {
                        ProjectCard { project: project.clone() }
                    }
                }
            }
        }
        Some(Err(e)) => {
            rsx! {
                ErrorMessage { message: format!("Failed to load projects: {}", e) }
            }
        }
        None => {
            rsx! {
                LoadingText { message: "Loading projects...".to_string() }
            }
        }
    };

    rsx! {
        div {
            class: "c-projects-grid",
            {content}
        }
    }
}





// Sessions List Component
#[component]
pub fn SessionsList(encoded_name: String) -> Element {
    let mut sessions = use_signal(Vec::<(String, Vec<Session>)>::new);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);
    let mut show_delete_all_confirm = use_signal(|| false);
    let mut deleting_all = use_signal(|| false);
    let refresh_trigger = use_signal(|| 0);
    let encoded_for_effect = encoded_name.clone();

    // Fetch sessions on mount and when refresh_trigger changes using server function
    // OPTIMIZED: Uses get_sessions_summaries() (metadata only) instead of get_histories() (all messages)
    use_effect(move || {
        let _ = refresh_trigger(); // Subscribe to changes
        let encoded = encoded_for_effect.clone();
        spawn(async move {
            loading.set(true);
            match get_sessions_summaries(encoded, None).await {
                Ok(summaries) => {
                    // Convert SessionSummaryResponse to Session (lightweight)
                    // DEBUG: Log first few summaries to understand the data
                    for (i, s) in summaries.iter().take(3).enumerate() {
                        tracing::info!(
                            "DEBUG Summary {}: id={}, updated_at='{}', preview_len={}",
                            i,
                            &s.session_id[..8.min(s.session_id.len())],
                            s.updated_at,
                            s.preview.len()
                        );
                    }
                    let mut session_list: Vec<Session> = summaries
                        .into_iter()
                        .map(|s| {
                            let parse_result = chrono::DateTime::parse_from_rfc3339(&s.updated_at);
                            if parse_result.is_err() {
                                tracing::warn!(
                                    "Failed to parse timestamp '{}' for session {}: {:?}",
                                    s.updated_at,
                                    &s.session_id[..8.min(s.session_id.len())],
                                    parse_result.err()
                                );
                            }
                            let updated = parse_result
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                .unwrap_or_else(|_| chrono::Utc::now());
                            Session {
                                id: s.session_id,
                                project_id: String::new(),
                                project_name: String::new(),
                                ai_tool: crate::domain::models::AiTool::ClaudeCode,
                                message_count: s.message_count,
                                summary: s.preview.clone(),
                                created_at: updated,
                                updated_at: updated,
                                status: crate::domain::models::SessionStatus::Completed,
                                last_message_preview: s.preview,
                                last_time: updated.format("%Y-%m-%d %H:%M").to_string(),
                            }
                        })
                        .collect();
                    // Sort by updated_at descending (most recent first)
                    session_list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                    let now = chrono::Utc::now();
                    let groups = group_sessions(session_list, &now);
                    sessions.set(groups);
                    loading.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load sessions: {}", e)));
                    loading.set(false);
                }
            }
        });
    });

    // Handle delete all sessions (TODO: implement server function for delete)
    let handle_delete_all = move |_| {
        deleting_all.set(true);
        // TODO: Implement delete_all_sessions server function
        tracing::warn!("Delete all sessions not yet implemented for fullstack");
        show_delete_all_confirm.set(false);
        deleting_all.set(false);
    };

    let total_sessions = sessions.read().iter().map(|(_, gs)| gs.len()).sum::<usize>();
    let show_delete = total_sessions > 0 && !*show_delete_all_confirm.read();

    rsx! {
        div {
            // Delete All Sessions Button
            if show_delete {
                div { class: "c-action-bar",
                    button {
                        class: "c-btn c-btn--destructive c-btn--sm",
                        onclick: move |_| {
                            tracing::info!("Button clicked!");
                            show_delete_all_confirm.set(true);
                        },
                        "🗑️ Delete All Sessions"
                    }
                }
            }

            // Confirmation Dialog
            if *show_delete_all_confirm.read() {
                div { class: "c-confirm-dialog",
                    p { class: "c-confirm-dialog__text",
                        "⚠️ Are you sure you want to delete ALL sessions?"
                    }
                    div { class: "c-confirm-dialog__actions",
                        button {
                            class: "c-btn c-btn--destructive c-btn--sm",
                            onclick: handle_delete_all,
                            disabled: *deleting_all.read(),
                            if *deleting_all.read() {
                                "Deleting..."
                            } else {
                                "Yes, Delete All"
                            }
                        }
                        button {
                            class: "c-btn c-btn--ghost c-btn--sm",
                            onclick: move |_| show_delete_all_confirm.set(false),
                            "Cancel"
                        }
                    }
                }
            }

            if *loading.read() {
                SessionsLoading {}
            } else if let Some(err) = error.read().as_ref() {
                ErrorMessage { message: err.clone() }
            } else if total_sessions == 0 {
                div { class: "c-sessions__empty",
                    div { class: "c-sessions__empty-icon", "💬" }
                    div { class: "c-sessions__empty-title", "Aucune session" }
                    div { class: "c-sessions__empty-description",
                        "Les sessions de conversation apparaîtront ici."
                    }
                }
            } else {
                div { class: "c-sessions",
                    for (group_name, group_sessions) in sessions.read().iter() {
                        if !group_sessions.is_empty() {
                            div { class: "c-sessions__group-header", "{group_name}" }
                            for session in group_sessions {
                                SessionCard {
                                    session: session.clone(),
                                    encoded_name: encoded_name.clone(),
                                    refresh_trigger: refresh_trigger
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SessionCard(session: Session, encoded_name: String, refresh_trigger: Signal<i32>) -> Element {
    use dioxus::prelude::*;

    let mut show_confirm = use_signal(|| false);
    let mut deleting = use_signal(|| false);
    let session_id_for_delete = session.id.clone();
    let encoded_for_delete = encoded_name.clone();
    let now = Utc::now();

    // Get tool slug from session's ai_tool
    let tool_slug = ai_tool_to_slug(&session.ai_tool).to_string();

    let handle_delete = move |_| {
        deleting.set(true);
        let encoded = encoded_for_delete.clone();
        let sid = session_id_for_delete.clone();
        spawn(async move {
            match delete_session(encoded, sid.clone()).await {
                Ok(success) => {
                    if success {
                        tracing::info!("Session deleted successfully");
                        // Trigger a refresh by incrementing the trigger
                        refresh_trigger.set(refresh_trigger() + 1);
                    } else {
                        tracing::warn!("Session not found for deletion");
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to delete session: {:?}", e);
                }
            }
            deleting.set(false);
            show_confirm.set(false);
        });
    };

    // Generate title from preview or session ID
    let title = if !session.summary.is_empty() {
        let preview = session.summary.trim();
        if preview.chars().count() > 60 {
            format!("{}...", preview.chars().take(60).collect::<String>())
        } else {
            preview.to_string()
        }
    } else {
        format!("Session {}", &session.id[..8.min(session.id.len())])
    };

    let relative_time = format_relative_time(&session.updated_at, &now);

    rsx! {
        if *show_confirm.read() {
            // Confirmation overlay - inline style for this overlay
            div {
                class: "c-session-item",
                div { class: "c-session-item__confirm-overlay",
                    span { class: "c-session-item__confirm-text", "Supprimer ?" }
                    button {
                        class: "c-session-item__confirm-btn c-session-item__confirm-btn--danger",
                        onclick: handle_delete,
                        disabled: *deleting.read(),
                        if *deleting.read() { "..." } else { "Oui" }
                    }
                    button {
                        class: "c-session-item__confirm-btn c-session-item__confirm-btn--cancel",
                        onclick: move |_| show_confirm.set(false),
                        "Non"
                    }
                }
            }
        } else {
            Link {
                class: "c-session-item",
                to: crate::app::pages::claude_routes::Route::Session {
                    tool: tool_slug.clone(),
                    project_name: encoded_name.clone(),
                    session_id: session.id.clone()
                },

                // Icon
                div { class: "c-session-item__icon", "💬" }

                // Content
                div { class: "c-session-item__content",
                    // Title
                    div { class: "c-session-item__title", "{title}" }
                    // Meta: message count
                    div { class: "c-session-item__meta",
                        span { "{session.message_count} messages" }
                    }
                }

                // Relative time
                div { class: "c-session-item__time", "{relative_time}" }

                // Delete button (shown on hover via CSS)
                button {
                    class: "c-session-item__delete",
                    onclick: move |evt: Event<MouseData>| {
                        evt.stop_propagation();
                        evt.prevent_default();
                        show_confirm.set(true);
                    },
                    "🗑️"
                }
            }
        }
    }
}

/// Sessions list with callback for session selection (used by Project page)
#[component]
pub fn SessionsListWithCallback(
    encoded_name: String,
    on_session_click: EventHandler<String>,
) -> Element {
    let mut sessions = use_signal(Vec::<(String, Vec<Session>)>::new);
    let mut loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);

    let encoded_for_effect = encoded_name.clone();

    // OPTIMIZED: Uses get_sessions_summaries() (metadata only) instead of get_histories() (all messages)
    use_effect(move || {
        let encoded = encoded_for_effect.clone();
        spawn(async move {
            loading.set(true);
            match get_sessions_summaries(encoded, None).await {
                Ok(summaries) => {
                    let mut session_list: Vec<Session> = summaries
                        .into_iter()
                        .map(|s| {
                            let updated = chrono::DateTime::parse_from_rfc3339(&s.updated_at)
                                .map(|dt| dt.with_timezone(&chrono::Utc))
                                .unwrap_or_else(|_| chrono::Utc::now());
                            Session {
                                id: s.session_id,
                                project_id: String::new(),
                                project_name: String::new(),
                                ai_tool: crate::domain::models::AiTool::ClaudeCode,
                                message_count: s.message_count,
                                summary: s.preview.clone(),
                                created_at: updated,
                                updated_at: updated,
                                status: crate::domain::models::SessionStatus::Completed,
                                last_message_preview: s.preview,
                                last_time: updated.format("%Y-%m-%d %H:%M").to_string(),
                            }
                        })
                        .collect();
                    // Sort by updated_at descending (most recent first)
                    session_list.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
                    let now = chrono::Utc::now();
                    let groups = group_sessions(session_list, &now);
                    sessions.set(groups);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load sessions: {}", e)));
                }
            }
            loading.set(false);
        });
    });

    rsx! {
        div { class: "c-sessions",
            if *loading.read() {
                SessionsLoading {}
            } else if let Some(err) = error.read().as_ref() {
                div { class: "c-sessions__empty",
                    div { class: "c-sessions__empty-icon", "⚠️" }
                    div { class: "c-sessions__empty-title", "Erreur" }
                    div { class: "c-sessions__empty-description", "{err}" }
                }
            } else if sessions.read().iter().all(|(_, gs)| gs.is_empty()) {
                div { class: "c-sessions__empty",
                    div { class: "c-sessions__empty-icon", "💬" }
                    div { class: "c-sessions__empty-title", "Aucune session" }
                    div { class: "c-sessions__empty-description", "Les conversations apparaîtront ici." }
                }
            } else {
                for (group_name, group_sessions) in sessions.read().iter() {
                    if !group_sessions.is_empty() {
                        div { class: "c-sessions__group-header", "{group_name}" }
                        for session in group_sessions {
                            SessionCardClickable {
                                session: session.clone(),
                                on_click: on_session_click,
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Clickable session card for SessionsListWithCallback - Claude.ai /recents style
#[component]
fn SessionCardClickable(session: Session, on_click: EventHandler<String>) -> Element {
    let session_id = session.id.clone();
    let now = chrono::Utc::now();
    let relative_time = format_relative_time(&session.updated_at, &now);

    // Title: first 60 chars of preview or session ID
    let title = if session.summary.is_empty() {
        format!("Session {}", &session.id[..8.min(session.id.len())])
    } else if session.summary.len() > 60 {
        format!("{}...", session.summary.chars().take(60).collect::<String>())
    } else {
        session.summary.clone()
    };

    rsx! {
        button {
            class: "c-session-item",
            onclick: move |_| on_click.call(session_id.clone()),

            // Icon
            div { class: "c-session-item__icon", "💬" }

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
    }
}

// Conversation View Component with Interactive Chat
// Uses SuspenseBoundary pattern for proper SSR hydration
#[component]
pub fn ConversationView(project_name: String, session_id: String) -> Element {
    rsx! {
        div { class: "c-conversation-view",
            SuspenseBoundary {
                fallback: |_| rsx! {
                    LoadingText { message: "Chargement de la conversation...".to_string() }
                },
                ConversationViewInner {
                    project_name: project_name.clone(),
                    session_id: session_id.clone()
                }
            }
        }
    }
}

// Inner component that uses the `?` pattern for proper Suspense integration
#[component]
fn ConversationViewInner(project_name: String, session_id: String) -> Element {
    // Clone for closures
    let project_clone = project_name.clone();
    let session_clone = session_id.clone();

    // Use server function with use_server_future and `?` for Suspense
    let conversation_result = use_server_future(move || {
        let p = project_clone.clone();
        let s = session_clone.clone();
        async move { crate::server_fns::get_conversation(p, s).await }
    })?;  // The `?` here integrates with SuspenseBoundary

    // Now we have the actual result (after loading is complete)
    // Save the result in a local variable to avoid lifetime issues with the temporary borrow
    let result = match &*conversation_result.read() {
        Some(Ok(Some(conv))) => rsx! {
            ConversationChat {
                project_name: project_name.clone(),
                session_id: session_id.clone(),
                initial_messages: conv.messages.clone()
            }
        },
        Some(Ok(None)) => rsx! {
            ErrorMessage { message: "Session introuvable".to_string() }
        },
        Some(Err(e)) => rsx! {
            ErrorMessage { message: format!("Erreur: {}", e) }
        },
        None => rsx! {
            // This shouldn't happen with SuspenseBoundary, but just in case
            LoadingText { message: "Chargement...".to_string() }
        }
    };
    result
}

// Interactive chat component with history + input
#[component]
fn ConversationChat(
    project_name: String,
    session_id: String,
    initial_messages: Vec<Message>,
) -> Element {
    // Chat state signals
    #[allow(unused_mut)]
    let mut messages = use_signal(|| initial_messages.clone());
    #[allow(unused_mut)]
    let mut input = use_signal(String::new);
    #[allow(unused_mut)]
    let mut is_loading = use_signal(|| false);
    #[allow(unused_variables)]
    let current_session_id = use_signal(|| Some(session_id.clone()));
    #[allow(unused_mut, unused_variables)]
    let mut current_request_id = use_signal(|| None::<String>);
    #[allow(unused_variables)]
    let current_assistant_message = use_signal(|| None::<Message>);
    #[allow(unused_mut, unused_variables)]
    let mut sse_status = use_signal(|| "connecting".to_string());
    #[allow(unused_variables)]
    let mounted = use_signal(|| true);

    // Image upload state
    let upload_state = crate::shared::hooks::use_image_upload();

    // CLI provider selection state
    let mut cli_provider = use_signal(|| super::CliProviderOption::Claude);

    // Working directory from project name (decode path)
    #[allow(unused_variables)]
    let working_directory = project_name.replace("-", "/");

    // SSE connection for real-time CLI → Web sync (WASM only)
    // Uses EventSource for instant updates when CLI writes new messages
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;

        let project_for_sse = project_name.clone();
        let session_for_sse = session_id.clone();

        use_effect(move || {
            let project = project_for_sse.clone();
            let session = session_for_sse.clone();

            // Build SSE URL
            let sse_url = format!("/api/sse/{}/{}", project, session);
            tracing::info!("Connecting to SSE: {}", sse_url);

            // Create EventSource connection
            let event_source = match EventSource::new(&sse_url) {
                Ok(es) => es,
                Err(e) => {
                    tracing::error!("Failed to create EventSource: {:?}", e);
                    sse_status.set("error".to_string());
                    return;
                }
            };

            // Handle connection open
            let mut sse_status_clone = sse_status.clone();
            let onopen = Closure::wrap(Box::new(move |_: web_sys::Event| {
                tracing::info!("SSE connection opened");
                sse_status_clone.set("connected".to_string());
            }) as Box<dyn FnMut(_)>);
            event_source.set_onopen(Some(onopen.as_ref().unchecked_ref()));
            onopen.forget();

            // Handle errors
            let mut sse_status_clone2 = sse_status.clone();
            let onerror = Closure::wrap(Box::new(move |_: web_sys::Event| {
                tracing::error!("SSE connection error");
                sse_status_clone2.set("error".to_string());
            }) as Box<dyn FnMut(_)>);
            event_source.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onerror.forget();

            // Handle "message" events (new messages from CLI)
            let messages_clone = messages.clone();
            let project_for_fetch = project.clone();
            let session_for_fetch = session.clone();
            let onmessage = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
                if let Some(data) = event.data().as_string() {
                    tracing::info!("SSE message received: {}", &data[..data.len().min(100)]);

                    // Parse SSE message to get new message count
                    if let Ok(sse_msg) = serde_json::from_str::<serde_json::Value>(&data) {
                        if sse_msg.get("event_type").and_then(|v| v.as_str()) == Some("new_messages") {
                            // Fetch full conversation to get properly parsed messages
                            let mut messages_inner = messages_clone.clone();
                            let project_inner = project_for_fetch.clone();
                            let session_inner = session_for_fetch.clone();

                            wasm_bindgen_futures::spawn_local(async move {
                                let api_url = format!("/api/projects/{}/histories/{}", project_inner, session_inner);

                                if let Some(window) = web_sys::window() {
                                    if let Ok(response) = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(&api_url)).await {
                                        if let Ok(response) = response.dyn_into::<web_sys::Response>() {
                                            if response.ok() {
                                                if let Ok(json) = wasm_bindgen_futures::JsFuture::from(response.json().unwrap()).await {
                                                    if let Ok(conv) = serde_wasm_bindgen::from_value::<crate::domain::models::Conversation>(json) {
                                                        let current_count = messages_inner.read().len();
                                                        let new_count = conv.messages.len();

                                                        if new_count > current_count {
                                                            tracing::info!("SSE: Adding {} new messages", new_count - current_count);
                                                            for msg in conv.messages.into_iter().skip(current_count) {
                                                                messages_inner.write().push(msg);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);
            event_source.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();

            // Store EventSource reference to close on cleanup
            // Note: EventSource will be closed when component unmounts due to page navigation
        });
    }

    // Send message handler (WASM only)
    #[cfg(target_arch = "wasm32")]
    let send_message = {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Request, RequestInit, RequestMode, Response};
        use crate::domain::models::ChatRequest;
        use crate::shared::utils::process_stream_line;
        use chrono::Utc;

        let working_dir = working_directory.clone();
        let cli_provider_clone = cli_provider.clone();
        let upload_state_for_send = upload_state.clone();

        move |message_content: String| {
            // LOG 1: Confirm closure is called
            web_sys::console::log_1(&"[WASM] 📤 send_message CALLED".into());

            let working_dir = working_dir.clone();
            let selected_cli = cli_provider_clone();
            let mut upload_state_clone = upload_state_for_send.clone();

            // Use spawn from dioxus::prelude instead of spawn_local (works in Dioxus EventHandler context)
            spawn(async move {
                // LOG 2: Confirm async block started
                web_sys::console::log_1(&"[WASM] 📤 dioxus::spawn STARTED".into());

                // Get images BEFORE clearing
                let image_paths = upload_state_clone.get_image_paths();
                // LOG 3: Show image paths
                web_sys::console::log_1(&format!("[WASM] 📸 image_paths: {:?}", image_paths).into());
                let uploaded_images: Vec<crate::domain::models::ImageAttachment> =
                    upload_state_clone.uploaded_images.read().clone();

                // 1. Add user message WITH images
                messages.write().push(Message::User {
                    content: message_content.clone(),
                    timestamp: Utc::now(),
                    images: uploaded_images,
                    metadata: None,
                });

                // 2. Generate request ID
                let request_id = format!("req-{}", uuid::Uuid::new_v4());
                current_request_id.set(Some(request_id.clone()));

                // 3. Clear input, images and start loading
                input.set(String::new());
                upload_state_clone.clear();
                is_loading.set(true);

                // 4. Build request
                let mut chat_request = ChatRequest::new(message_content.clone(), request_id.clone())
                    .with_working_directory(working_dir);

                // Add session_id for continuation
                if let Some(sid) = (*current_session_id.read()).clone() {
                    chat_request = chat_request.with_session_id(sid);
                }

                // Add CLI provider
                chat_request = chat_request.with_cli_provider(selected_cli.slug().to_string());

                // Add images if any
                if !image_paths.is_empty() {
                    web_sys::console::log_1(&format!("[WASM] 📸 Adding {} images to request", image_paths.len()).into());
                    chat_request = chat_request.with_images(image_paths);
                } else {
                    web_sys::console::log_1(&"[WASM] 📸 No images to add".into());
                }

                let request_body = match serde_json::to_string(&chat_request) {
                    Ok(body) => {
                        web_sys::console::log_1(&format!("[WASM] 📤 Request body length: {} bytes", body.len()).into());
                        body
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("[WASM] ❌ Failed to serialize request: {}", e).into());
                        is_loading.set(false);
                        return;
                    }
                };

                // 5. Send request
                let window = match web_sys::window() {
                    Some(w) => w,
                    None => {
                        is_loading.set(false);
                        return;
                    }
                };

                let mut opts = RequestInit::new();
                opts.method("POST");
                opts.mode(RequestMode::SameOrigin);
                opts.body(Some(&JsValue::from_str(&request_body)));

                let request = match Request::new_with_str_and_init("/api/chat/native", &opts) {
                    Ok(req) => req,
                    Err(_) => {
                        is_loading.set(false);
                        return;
                    }
                };

                let _ = request.headers().set("Content-Type", "application/json");

                web_sys::console::log_1(&"[WASM] 📤 Sending fetch request to /api/chat/native...".into());
                let resp_promise = window.fetch_with_request(&request);
                let resp_value = match JsFuture::from(resp_promise).await {
                    Ok(v) => {
                        web_sys::console::log_1(&"[WASM] ✅ Fetch response received".into());
                        v
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("[WASM] ❌ Fetch error: {:?}", e).into());
                        is_loading.set(false);
                        return;
                    }
                };

                let response: Response = resp_value.dyn_into().unwrap();
                web_sys::console::log_1(&format!("[WASM] 📥 Response status: {}", response.status()).into());

                if !response.ok() {
                    web_sys::console::error_1(&format!("[WASM] ❌ Response not OK: {}", response.status()).into());
                    is_loading.set(false);
                    return;
                }

                // 6. Stream response
                let body = match response.body() {
                    Some(b) => b,
                    None => {
                        is_loading.set(false);
                        return;
                    }
                };

                let reader = body.get_reader().dyn_into::<web_sys::ReadableStreamDefaultReader>().unwrap();
                let mut buffer = String::new();

                web_sys::console::log_1(&"[WASM] 🔄 Starting stream read loop".into());
                let mut chunk_count = 0u32;

                loop {
                    let result = match JsFuture::from(reader.read()).await {
                        Ok(r) => r,
                        Err(e) => {
                            web_sys::console::error_1(&format!("[WASM] ❌ Stream read error: {:?}", e).into());
                            break;
                        }
                    };

                    let done = js_sys::Reflect::get(&result, &JsValue::from_str("done"))
                        .unwrap_or(JsValue::TRUE)
                        .as_bool()
                        .unwrap_or(true);

                    if done {
                        web_sys::console::log_1(&format!("[WASM] ✅ Stream done after {} chunks", chunk_count).into());
                        break;
                    }

                    let value = js_sys::Reflect::get(&result, &JsValue::from_str("value"))
                        .ok()
                        .and_then(|v| v.dyn_into::<js_sys::Uint8Array>().ok());

                    if let Some(chunk) = value {
                        chunk_count += 1;
                        let bytes = chunk.to_vec();
                        web_sys::console::log_1(&format!("[WASM] 📦 Chunk #{}: {} bytes", chunk_count, bytes.len()).into());

                        if let Ok(text) = String::from_utf8(bytes) {
                            web_sys::console::log_1(&format!("[WASM] 📝 Text: {}", &text[..text.len().min(200)]).into());
                            buffer.push_str(&text);

                            while let Some(newline_pos) = buffer.find('\n') {
                                let line = buffer[..newline_pos].to_string();
                                buffer = buffer[newline_pos + 1..].to_string();

                                if line.trim().is_empty() {
                                    continue;
                                }

                                web_sys::console::log_1(&format!("[WASM] 📄 Processing line: {}", &line[..line.len().min(100)]).into());
                                process_stream_line(
                                    &line,
                                    messages,
                                    current_assistant_message,
                                    current_session_id,
                                    is_loading,
                                );
                            }
                        }
                    }
                }

                // Process remaining buffer
                if !buffer.trim().is_empty() {
                    process_stream_line(
                        &buffer,
                        messages,
                        current_assistant_message,
                        current_session_id,
                        is_loading,
                    );
                }

                is_loading.set(false);
            });
        }
    };

    // Abort handler (WASM only)
    #[cfg(target_arch = "wasm32")]
    let abort_request = {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen_futures::{spawn_local, JsFuture};
        use web_sys::{Request, RequestInit, RequestMode};

        move || {
            if let Some(request_id) = (*current_request_id.read()).clone() {
                spawn_local(async move {
                    let url = format!("/api/abort/{}", request_id);

                    if let Some(window) = web_sys::window() {
                        let mut opts = RequestInit::new();
                        opts.method("POST");
                        opts.mode(RequestMode::SameOrigin);

                        if let Ok(request) = Request::new_with_str_and_init(&url, &opts) {
                            let resp_promise = window.fetch_with_request(&request);
                            let _ = JsFuture::from(resp_promise).await;
                        }
                    }

                    is_loading.set(false);
                    current_request_id.set(None);
                });
            }
        }
    };

    // Submit handler - WASM only does the actual work
    #[cfg(target_arch = "wasm32")]
    let on_submit_handler = {
        let upload_state_for_submit = upload_state.clone();
        move |_| {
            let input_value = (*input.read()).clone();
            let has_images = upload_state_for_submit.has_images();
            // Allow sending if text is not empty OR if there are images
            if (!input_value.trim().is_empty() || has_images) && !*is_loading.read() {
                send_message(input_value);
            }
        }
    };

    #[cfg(not(target_arch = "wasm32"))]
    let on_submit_handler = move |_| {
        // SSR: no-op, WASM will hydrate with real handler
    };

    // Abort handler - WASM only does the actual work
    #[cfg(target_arch = "wasm32")]
    let on_abort_handler = move |_| abort_request();

    #[cfg(not(target_arch = "wasm32"))]
    let on_abort_handler = move |_| {
        // SSR: no-op, WASM will hydrate with real handler
    };

    // Auto-scroll to bottom when messages change (WASM only)
    let messages_len = messages.read().len();
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let script = r#"
                setTimeout(() => {
                    // Scroll the messages container to bottom (not the window)
                    const messagesContainer = document.querySelector('.c-conversation-chat__messages');
                    if (messagesContainer) {
                        messagesContainer.scrollTop = messagesContainer.scrollHeight;
                    }
                    // Also focus the input
                    const input = document.getElementById('chat-input');
                    if (input) {
                        input.focus();
                    }
                }, 150);
            "#;
            let _ = js_sys::eval(script);
        }
        let _ = messages_len; // Use the variable to trigger effect on messages change
    });

    rsx! {
        div { class: "c-conversation-chat",
            // Messages area - scrollable
            div { class: "c-conversation-chat__messages",
                ul { class: "c-conversation-list",
                    for message in messages.read().iter().filter(|m| !is_message_empty(m)) {
                        MessageItem { message: message.clone() }
                    }
                }
            }

            // Chat input - fixed at bottom
            div { class: "c-conversation-chat__input",
                super::chat_input::ChatInput {
                    input: input,
                    is_loading: is_loading,
                    upload_state: upload_state.clone(),
                    on_submit: on_submit_handler,
                    on_abort: on_abort_handler,
                }
            }
        }
    }
}

// Image Upload Component (WASM-only - uses web_sys and Request directly)
#[cfg(target_arch = "wasm32")]
#[component]
pub fn ImageUpload() -> Element {
    let selected_file = use_signal(|| None::<String>);
    let mut upload_result = use_signal(|| None::<crate::domain::models::ImageUpload>);
    let mut analysis_prompt = use_signal(|| String::from("Décris cette image en détail"));
    let mut analysis_result = use_signal(|| None::<String>);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let handle_file_change = move |evt: Event<FormData>| {
        // TODO: Implement file selection
        // if let Some(file_engine) = evt.files() {
        //     if let Some(file) = file_engine.files().get(0) {
        //         selected_file.set(Some(file.to_string()));
        //     }
        // }
    };

    let handle_upload = move |_| {
        if selected_file().is_some() {
            loading.set(true);
            error.set(None);

            spawn(async move {
                // Create FormData
                let form_data = web_sys::FormData::new().unwrap();

                // Get file from input
                let document = web_sys::window().unwrap().document().unwrap();
                let input: HtmlElement = document
                    .get_element_by_id("file-input")
                    .unwrap()
                    .dyn_into()
                    .unwrap();

                // TODO: Implement file upload
                // if let Some(files) = input.files() {
                //     if let Some(file) = files.get(0) {
                //         form_data.append_with_blob("file", &file).unwrap();
                //     }
                // }

                // Upload
                match Request::post("/api/upload").body(form_data).unwrap().send().await {
                    Ok(response) => match response.json::<crate::domain::models::ImageUpload>().await {
                        Ok(data) => {
                            upload_result.set(Some(data));
                            loading.set(false);
                        }
                        Err(e) => {
                            error.set(Some(format!("Parse error: {}", e)));
                            loading.set(false);
                        }
                    },
                    Err(e) => {
                        error.set(Some(format!("Upload error: {}", e)));
                        loading.set(false);
                    }
                }
            });
        }
    };

    let handle_analyze = move |_| {
        if let Some(upload) = upload_result() {
            loading.set(true);
            error.set(None);

            let request = crate::domain::models::ImageAnalysisRequest {
                image_id: upload.id.clone(),
                prompt: analysis_prompt(),
            };

            spawn(async move {
                let json_body = serde_json::to_string(&request).unwrap();

                match Request::post("/api/analyze")
                    .header("content-type", "application/json")
                    .body(json_body)
                    .unwrap()
                    .send()
                    .await
                {
                    Ok(response) => match response.json::<crate::domain::models::ImageAnalysisResponse>().await {
                        Ok(data) => {
                            analysis_result.set(Some(data.analysis));
                            loading.set(false);
                        }
                        Err(e) => {
                            error.set(Some(format!("Parse error: {}", e)));
                            loading.set(false);
                        }
                    },
                    Err(e) => {
                        error.set(Some(format!("Analysis error: {}", e)));
                        loading.set(false);
                    }
                }
            });
        }
    };

    rsx! {
        div { class: "image-upload-container",
            h2 { "Vision Analysis avec Claude" }

            div { class: "upload-section",
                input {
                    id: "file-input",
                    r#type: "file",
                    accept: "image/*",
                    onchange: handle_file_change,
                }

                if selected_file().is_some() {
                    button {
                        onclick: handle_upload,
                        disabled: *loading.read(),
                        "Upload Image"
                    }
                }
            }

            if let Some(err) = error.read().as_ref() {
                p { class: "error", {err.clone()} }
            }

            if *loading.read() {
                p { "Loading..." }
            }

            if let Some(upload) = upload_result.read().as_ref() {
                div { class: "upload-result",
                    h3 { "Image uploadée" }
                    img {
                        src: "/api/image/{upload.id}",
                        alt: "{upload.filename}",
                        style: "max-width: 500px",
                    }
                    p { "Nom: {upload.filename}" }
                    p { "Taille: {upload.size} octets" }

                    div { class: "analysis-section",
                        h3 { "Analyser l'image" }
                        textarea {
                            value: "{analysis_prompt}",
                            oninput: move |evt| analysis_prompt.set(evt.value()),
                            rows: "3",
                            placeholder: "Entrez votre question...",
                        }
                        button {
                            onclick: handle_analyze,
                            disabled: *loading.read(),
                            "Analyser"
                        }
                    }
                }
            }

            if let Some(analysis) = analysis_result.read().as_ref() {
                div { class: "analysis-result",
                    h3 { "Résultat de l'analyse" }
                    pre { {analysis.clone()} }
                }
            }
        }
    }
}
