use dioxus::prelude::*;

// Helper function to extract command name from pattern like "Bash(ls:*)" -> "ls"
fn extract_command_name(pattern: &str) -> String {
    if pattern.is_empty() {
        return "Unknown".to_string();
    }

    // Try to extract command from "Bash(command:*)" format
    if let Some(start) = pattern.find("Bash(") {
        if let Some(end) = pattern[start..].find(':') {
            let cmd_start = start + 5; // length of "Bash("
            let cmd_end = start + end;
            return pattern[cmd_start..cmd_end].to_string();
        }
    }

    pattern.to_string()
}

// Helper function to render permission content
fn render_permission_content(patterns: &[String]) -> Element {
    if patterns.is_empty() {
        rsx! {
            p { class: "text-slate-600 ",
                "Claude wants to use bash commands, but the specific commands could not be determined."
            }
        }
    } else if patterns.len() > 1 {
        let command_names: Vec<String> = patterns.iter()
            .map(|p| extract_command_name(p))
            .collect();

        rsx! {
            div {
                p { class: "text-slate-600 ",
                    "Claude wants to use the following commands:"
                }
                div { class: "flex flex-wrap gap-2 mb-3",
                    {command_names.iter().map(|cmd| rsx! {
                        span { class: "font-mono bg-slate-100 ",
                            "{cmd}"
                        }
                    })}
                }
            }
        }
    } else {
        let command_name = extract_command_name(&patterns[0]);
        rsx! {
            p { class: "text-slate-600 ",
                "Claude wants to use the "
                span { class: "font-mono bg-slate-100 ",
                    "{command_name}"
                }
                " command."
            }
        }
    }
}

// Helper function for permanent button text
fn render_permanent_button_text(patterns: &[String]) -> String {
    if patterns.is_empty() {
        return "Yes, and don't ask again for bash commands".to_string();
    }

    let command_names: Vec<String> = patterns.iter()
        .map(|p| extract_command_name(p))
        .collect();

    if patterns.len() > 1 {
        format!("Yes, and don't ask again for {} commands", command_names.join(" and "))
    } else {
        format!("Yes, and don't ask again for {} command", command_names[0])
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct PermissionDialogProps {
    pub patterns: Vec<String>,
    pub on_allow: EventHandler<()>,
    pub on_allow_permanent: EventHandler<()>,
    pub on_deny: EventHandler<()>,
}

#[component]
pub fn PermissionDialog(props: PermissionDialogProps) -> Element {
    let mut selected_option = use_signal(|| Some("allow".to_string()));

    // Handle keyboard navigation
    let handle_keydown = move |evt: Event<KeyboardData>| {
        match evt.key() {
            Key::ArrowDown => {
                evt.prevent_default();
                let current = selected_option.read().as_ref().unwrap_or(&"allow".to_string()).clone();
                let next = match current.as_str() {
                    "allow" => "allow_permanent",
                    "allow_permanent" => "deny",
                    _ => "allow",
                };
                selected_option.set(Some(next.to_string()));
            }
            Key::ArrowUp => {
                evt.prevent_default();
                let current = selected_option.read().as_ref().unwrap_or(&"allow".to_string()).clone();
                let prev = match current.as_str() {
                    "deny" => "allow_permanent",
                    "allow_permanent" => "allow",
                    _ => "deny",
                };
                selected_option.set(Some(prev.to_string()));
            }
            Key::Enter => {
                evt.prevent_default();
                let current = selected_option.read().as_ref().unwrap_or(&"allow".to_string()).clone();
                match current.as_str() {
                    "allow" => props.on_allow.call(()),
                    "allow_permanent" => props.on_allow_permanent.call(()),
                    "deny" => props.on_deny.call(()),
                    _ => {}
                }
            }
            Key::Escape => {
                evt.prevent_default();
                props.on_deny.call(());
            }
            _ => {}
        }
    };

    let is_selected = |option: &str| {
        selected_option.read().as_ref().is_some_and(|s| s == option)
    };

    rsx! {
        div {
            class: "flex-shrink-0 px-4 py-4 bg-white/80 dark:bg-slate-800/80 ",
            onkeydown: handle_keydown,

            // Header
            div { class: "flex items-center gap-3 mb-4",
                div { class: "p-2 bg-amber-100 dark:bg-amber-900/20 ",
                    // Warning icon SVG
                    svg {
                        class: "w-5 h-5 text-amber-600 dark:text-amber-400 ",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                        }
                    }
                }
                h3 { class: "text-lg font-semibold text-slate-800 dark:text-slate-200 dark:text-slate-100 ",
                    "Permission Required"
                }
            }

            // Content
            div { class: "mb-4",
                {render_permission_content(&props.patterns)}
                p { class: "text-sm text-slate-500 dark:text-slate-400 ",
                    "Do you want to proceed? (Press ESC to deny)"
                }
            }

            // Options
            div { class: "space-y-2",
                // Allow button
                button {
                    class: format!(
                        "w-full p-3 rounded-lg cursor-pointer transition-all duration-200 text-left focus:outline-none {}",
                        if is_selected("allow") {
                            "bg-blue-50 dark:bg-blue-900/20 "
                        } else {
                            "border-2 border-transparent"
                        }
                    ),
                    onclick: move |_| {
                        selected_option.set(Some("allow".to_string()));
                        props.on_allow.call(());
                    },
                    onmouseenter: move |_| selected_option.set(Some("allow".to_string())),
                    onmouseleave: move |_| selected_option.set(None),

                    span {
                        class: format!(
                            "text-sm font-medium {}",
                            if is_selected("allow") {
                                "text-blue-700 dark:text-blue-300 "
                            } else {
                                "text-slate-700 dark:text-slate-300 "
                            }
                        ),
                        "Yes"
                    }
                }

                // Allow permanent button
                button {
                    class: format!(
                        "w-full p-3 rounded-lg cursor-pointer transition-all duration-200 text-left focus:outline-none {}",
                        if is_selected("allow_permanent") {
                            "bg-green-50 dark:bg-green-900/20 "
                        } else {
                            "border-2 border-transparent"
                        }
                    ),
                    onclick: move |_| {
                        selected_option.set(Some("allow_permanent".to_string()));
                        props.on_allow_permanent.call(());
                    },
                    onmouseenter: move |_| selected_option.set(Some("allow_permanent".to_string())),
                    onmouseleave: move |_| selected_option.set(None),

                    span {
                        class: format!(
                            "text-sm font-medium {}",
                            if is_selected("allow_permanent") {
                                "text-green-700 dark:text-green-300 "
                            } else {
                                "text-slate-700 dark:text-slate-300 "
                            }
                        ),
                        "{render_permanent_button_text(&props.patterns)}"
                    }
                }

                // Deny button
                button {
                    class: format!(
                        "w-full p-3 rounded-lg cursor-pointer transition-all duration-200 text-left focus:outline-none {}",
                        if is_selected("deny") {
                            "bg-slate-50 dark:bg-slate-800 dark:bg-slate-900/50 "
                        } else {
                            "border-2 border-transparent"
                        }
                    ),
                    onclick: move |_| {
                        selected_option.set(Some("deny".to_string()));
                        props.on_deny.call(());
                    },
                    onmouseenter: move |_| selected_option.set(Some("deny".to_string())),
                    onmouseleave: move |_| selected_option.set(None),

                    span {
                        class: format!(
                            "text-sm font-medium {}",
                            if is_selected("deny") {
                                "text-slate-800 dark:text-slate-200 dark:text-slate-100 "
                            } else {
                                "text-slate-700 dark:text-slate-300 "
                            }
                        ),
                        "No"
                    }
                }
            }
        }
    }
}
