use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct PlanModeDialogProps {
    pub plan_content: String,
    pub on_accept_with_edits: EventHandler<()>,
    pub on_accept_default: EventHandler<()>,
    pub on_keep_planning: EventHandler<()>,
}

#[component]
pub fn PlanModeDialog(props: PlanModeDialogProps) -> Element {
    let mut selected_option = use_signal(|| Some("accept_with_edits".to_string()));

    // Handle keyboard navigation
    let handle_keydown = move |evt: Event<KeyboardData>| {
        match evt.key() {
            Key::ArrowDown => {
                evt.prevent_default();
                let current = selected_option.read().as_ref().unwrap_or(&"accept_with_edits".to_string()).clone();
                let next = match current.as_str() {
                    "accept_with_edits" => "accept_default",
                    "accept_default" => "keep_planning",
                    _ => "accept_with_edits",
                };
                selected_option.set(Some(next.to_string()));
            }
            Key::ArrowUp => {
                evt.prevent_default();
                let current = selected_option.read().as_ref().unwrap_or(&"accept_with_edits".to_string()).clone();
                let prev = match current.as_str() {
                    "keep_planning" => "accept_default",
                    "accept_default" => "accept_with_edits",
                    _ => "keep_planning",
                };
                selected_option.set(Some(prev.to_string()));
            }
            Key::Enter => {
                evt.prevent_default();
                let current = selected_option.read().as_ref().unwrap_or(&"accept_with_edits".to_string()).clone();
                match current.as_str() {
                    "accept_with_edits" => props.on_accept_with_edits.call(()),
                    "accept_default" => props.on_accept_default.call(()),
                    "keep_planning" => props.on_keep_planning.call(()),
                    _ => {}
                }
            }
            Key::Escape => {
                evt.prevent_default();
                props.on_keep_planning.call(());
            }
            _ => {}
        }
    };

    let is_selected = |option: &str| {
        selected_option.read().as_ref().is_some_and(|s| s == option)
    };

    rsx! {
        div {
            class: "flex-shrink-0 px-4 py-4 bg-white/80 dark:bg-slate-800/80 dark:bg-slate-800/80 border border-slate-200 dark:border-slate-700 dark:border-slate-700 dark:border-slate-700 rounded-xl backdrop-blur-sm shadow-sm",
            onkeydown: handle_keydown,

            // Header
            div { class: "flex items-center gap-3 mb-4",
                div { class: "p-2 bg-blue-100 dark:bg-blue-900/20 ",
                    // Plan icon SVG
                    svg {
                        class: "w-5 h-5 text-blue-600 dark:text-blue-400 ",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"
                        }
                    }
                }
                h3 { class: "text-lg font-semibold text-slate-800 dark:text-slate-200 dark:text-slate-100 ",
                    "Plan Mode"
                }
            }

            // Plan content preview
            if !props.plan_content.is_empty() {
                div { class: "mb-4 p-3 bg-slate-50 dark:bg-slate-800 dark:bg-slate-900/50 ",
                    p { class: "text-sm text-slate-700 dark:text-slate-300 ",
                        "{props.plan_content.chars().take(200).collect::<String>()}"
                        {if props.plan_content.len() > 200 { "..." } else { "" }}
                    }
                }
            }

            // Content
            div { class: "mb-4",
                p { class: "text-sm text-slate-500 dark:text-slate-400 ",
                    "Choose how to proceed (Press ESC to keep planning)"
                }
            }

            // Options
            div { class: "space-y-2",
                // Accept with edits button
                button {
                    class: format!(
                        "w-full p-3 rounded-lg cursor-pointer transition-all duration-200 text-left focus:outline-none {}",
                        if is_selected("accept_with_edits") {
                            "bg-green-50 dark:bg-green-900/20 "
                        } else {
                            "border-2 border-transparent"
                        }
                    ),
                    onclick: move |_| {
                        selected_option.set(Some("accept_with_edits".to_string()));
                        props.on_accept_with_edits.call(());
                    },
                    onmouseenter: move |_| selected_option.set(Some("accept_with_edits".to_string())),
                    onmouseleave: move |_| selected_option.set(None),

                    span {
                        class: format!(
                            "text-sm font-medium {}",
                            if is_selected("accept_with_edits") {
                                "text-green-700 dark:text-green-300 "
                            } else {
                                "text-slate-700 dark:text-slate-300 "
                            }
                        ),
                        "Yes, and auto-accept edits"
                    }
                }

                // Accept default button
                button {
                    class: format!(
                        "w-full p-3 rounded-lg cursor-pointer transition-all duration-200 text-left focus:outline-none {}",
                        if is_selected("accept_default") {
                            "bg-blue-50 dark:bg-blue-900/20 "
                        } else {
                            "border-2 border-transparent"
                        }
                    ),
                    onclick: move |_| {
                        selected_option.set(Some("accept_default".to_string()));
                        props.on_accept_default.call(());
                    },
                    onmouseenter: move |_| selected_option.set(Some("accept_default".to_string())),
                    onmouseleave: move |_| selected_option.set(None),

                    span {
                        class: format!(
                            "text-sm font-medium {}",
                            if is_selected("accept_default") {
                                "text-blue-700 dark:text-blue-300 "
                            } else {
                                "text-slate-700 dark:text-slate-300 "
                            }
                        ),
                        "Yes, and manually approve edits"
                    }
                }

                // Keep planning button
                button {
                    class: format!(
                        "w-full p-3 rounded-lg cursor-pointer transition-all duration-200 text-left focus:outline-none {}",
                        if is_selected("keep_planning") {
                            "bg-slate-50 dark:bg-slate-800 dark:bg-slate-900/50 "
                        } else {
                            "border-2 border-transparent"
                        }
                    ),
                    onclick: move |_| {
                        selected_option.set(Some("keep_planning".to_string()));
                        props.on_keep_planning.call(());
                    },
                    onmouseenter: move |_| selected_option.set(Some("keep_planning".to_string())),
                    onmouseleave: move |_| selected_option.set(None),

                    span {
                        class: format!(
                            "text-sm font-medium {}",
                            if is_selected("keep_planning") {
                                "text-slate-800 dark:text-slate-200 dark:text-slate-100 "
                            } else {
                                "text-slate-700 dark:text-slate-300 "
                            }
                        ),
                        "No, keep planning"
                    }
                }
            }
        }
    }
}
