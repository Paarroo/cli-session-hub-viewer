//! Search bar component
//!
//! WASM-only - uses Request directly for API calls

#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use gloo_net::http::Request;

/// Search bar component for searching across conversations
#[cfg(target_arch = "wasm32")]
#[component]
pub fn SearchBar() -> Element {
    let mut search_query = use_signal(|| String::new());
    let mut selected_ai_tool = use_signal(|| String::from("all"));
    let mut search_results = use_signal(|| Vec::<crate::domain::models::SearchResult>::new());
    let mut is_searching = use_signal(|| false);

    let mut handle_search = move |_| {
        if search_query().trim().is_empty() {
            return;
        }

        is_searching.set(true);
        let query = search_query().clone();
        let ai_filter = selected_ai_tool().clone();

        spawn(async move {
            let search_request = crate::domain::models::SearchQuery {
                query: query.clone(),
                ai_tool: match ai_filter.as_str() {
                    "claude" => Some(crate::domain::models::AiTool::ClaudeCode),
                    "opencode" => Some(crate::domain::models::AiTool::OpenCode),
                    "gemini" => Some(crate::domain::models::AiTool::Gemini),
                    _ => None,
                },
                project_id: None,
                date_from: None,
                date_to: None,
                limit: Some(20),
                offset: None,
            };

            match Request::post("/api/search")
                .header("content-type", "application/json")
                .body(serde_json::to_string(&search_request).unwrap())
                .unwrap()
                .send()
                .await
            {
                Ok(response) => match response.json::<Vec<crate::domain::models::SearchResult>>().await {
                    Ok(results) => {
                        search_results.set(results);
                    }
                    Err(e) => {
                        tracing::error!("Search parse error: {}", e);
                    }
                },
                Err(e) => {
                    tracing::error!("Search request error: {}", e);
                }
            }

            is_searching.set(false);
        });
    };

    rsx! {
        div { class: "c-search-bar",
            h3 { class: "c-search-bar__title",
                "🔍 Search Conversations"
            }

            div { class: "c-search-bar__form",
                input {
                    r#type: "text",
                    class: "c-search-bar__input",
                    placeholder: "Search in all conversations...",
                    value: "{search_query}",
                    oninput: move |evt| search_query.set(evt.value()),
                    onkeypress: move |evt| {
                        if evt.key() == Key::Enter {
                            handle_search(());
                        }
                    },
                }

                select {
                    class: "c-search-bar__select",
                    value: "{selected_ai_tool}",
                    onchange: move |evt| selected_ai_tool.set(evt.value()),
                    option { value: "all", "All AI Tools" }
                    option { value: "claude", "🤖 Claude Code" }
                    option { value: "opencode", "🚀 OpenCode" }
                }

                button {
                    class: "c-btn c-btn--primary c-btn--sm",
                    onclick: move |_| handle_search(()),
                    disabled: *is_searching.read() || search_query().trim().is_empty(),
                    if *is_searching.read() {
                        "Searching..."
                    } else {
                        "Search"
                    }
                }
            }

            if !search_results.read().is_empty() {
                div { class: "c-search-results",
                    h4 { class: "c-search-results__title",
                        "Search Results ({search_results.len()})"
                    }

                    for result in search_results.read().iter() {
                        SearchResultItem { result: result.clone() }
                    }
                }
            }
        }
    }
}

/// Single search result item
#[cfg(target_arch = "wasm32")]
#[component]
fn SearchResultItem(result: crate::domain::models::SearchResult) -> Element {
    let _ai_icon = match result.ai_tool {
        crate::domain::models::AiTool::ClaudeCode => "🤖",
        crate::domain::models::AiTool::OpenCode => "🚀",
        crate::domain::models::AiTool::Gemini => "🧠",
    };

    rsx! {
        div { "Search result: {result.session_id}" }
    }
}
