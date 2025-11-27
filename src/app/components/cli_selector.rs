//! CLI Provider Selector Component
//!
//! Dropdown component for selecting which CLI provider to use for executing commands.
//! Supports Claude, OpenCode, and Gemini CLI with image capability indicators.

use dioxus::prelude::*;

/// CLI provider options with display info
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum CliProviderOption {
    #[default]
    Claude,
    OpenCode,
    Gemini,
}

impl CliProviderOption {
    /// All available providers
    pub const ALL: [CliProviderOption; 3] = [
        CliProviderOption::Claude,
        CliProviderOption::OpenCode,
        CliProviderOption::Gemini,
    ];

    /// Display name for the provider
    pub fn display_name(&self) -> &'static str {
        match self {
            CliProviderOption::Claude => "Claude",
            CliProviderOption::OpenCode => "OpenCode",
            CliProviderOption::Gemini => "Gemini",
        }
    }

    /// Icon for the provider
    pub fn icon(&self) -> &'static str {
        match self {
            CliProviderOption::Claude => "🤖",
            CliProviderOption::OpenCode => "⚡",
            CliProviderOption::Gemini => "🧠",
        }
    }

    /// Whether the provider supports images in headless mode
    pub fn supports_images(&self) -> bool {
        match self {
            CliProviderOption::Claude => true,
            CliProviderOption::OpenCode => true,
            CliProviderOption::Gemini => false, // TUI only - no headless image support
        }
    }

    /// Image support display string
    pub fn image_support_display(&self) -> &'static str {
        match self {
            CliProviderOption::Claude => "✅",
            CliProviderOption::OpenCode => "✅",
            CliProviderOption::Gemini => "❌",
        }
    }

    /// Explanation for image support status
    pub fn image_support_tooltip(&self) -> &'static str {
        match self {
            CliProviderOption::Claude => "Images supportées via --image",
            CliProviderOption::OpenCode => "Images supportées via --file",
            CliProviderOption::Gemini => "TUI only - Les images ne sont supportées que dans l'interface TUI interactive (drag & drop), pas en mode headless",
        }
    }

    /// Slug identifier
    pub fn slug(&self) -> &'static str {
        match self {
            CliProviderOption::Claude => "claude",
            CliProviderOption::OpenCode => "opencode",
            CliProviderOption::Gemini => "gemini",
        }
    }

    /// Create from slug
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "claude" => Some(CliProviderOption::Claude),
            "opencode" => Some(CliProviderOption::OpenCode),
            "gemini" => Some(CliProviderOption::Gemini),
            _ => None,
        }
    }
}


/// CLI Provider Selector dropdown component
#[component]
pub fn CliSelector(
    /// Currently selected provider
    selected: Signal<CliProviderOption>,
    /// Callback when selection changes
    on_change: EventHandler<CliProviderOption>,
    /// Whether the selector is disabled
    #[props(default = false)]
    disabled: bool,
    /// Show image support indicator
    #[props(default = true)]
    show_image_support: bool,
) -> Element {
    let current = selected();

    rsx! {
        div { class: "c-cli-selector",
            select {
                class: "c-cli-selector__select",
                disabled: disabled,
                value: "{current.slug()}",
                onchange: move |evt| {
                    if let Some(provider) = CliProviderOption::from_slug(&evt.value()) {
                        on_change.call(provider);
                    }
                },
                for provider in CliProviderOption::ALL {
                    option {
                        value: "{provider.slug()}",
                        selected: provider == current,
                        "{provider.icon()} {provider.display_name()}"
                        if show_image_support && !provider.supports_images() {
                            " (no images)"
                        }
                    }
                }
            }

            // Image support indicator with tooltip
            if show_image_support {
                span {
                    class: if current.supports_images() {
                        "c-cli-selector__indicator c-cli-selector__indicator--supports"
                    } else {
                        "c-cli-selector__indicator c-cli-selector__indicator--no-support"
                    },
                    title: "{current.image_support_tooltip()}",
                    "{current.image_support_display()} 🖼️"
                }
            }
        }
    }
}

/// Compact CLI selector for use in toolbars
#[component]
pub fn CliSelectorCompact(
    selected: Signal<CliProviderOption>,
    on_change: EventHandler<CliProviderOption>,
    #[props(default = false)]
    disabled: bool,
) -> Element {
    let current = selected();

    rsx! {
        button {
            class: "c-cli-selector-compact",
            disabled: disabled,
            title: "Select CLI provider",
            onclick: move |_| {
                // Cycle through providers on click
                let next = match current {
                    CliProviderOption::Claude => CliProviderOption::OpenCode,
                    CliProviderOption::OpenCode => CliProviderOption::Gemini,
                    CliProviderOption::Gemini => CliProviderOption::Claude,
                };
                on_change.call(next);
            },
            span { class: "c-cli-selector-compact__icon", "{current.icon()}" }
            span { class: "c-cli-selector-compact__name", "{current.display_name()}" }
        }
    }
}

/// CLI selector with availability indicators
#[component]
pub fn CliSelectorWithStatus(
    selected: Signal<CliProviderOption>,
    on_change: EventHandler<CliProviderOption>,
    /// Map of provider slug -> is_available
    available_providers: Vec<String>,
    #[props(default = false)]
    disabled: bool,
) -> Element {
    let current = selected();

    rsx! {
        div { class: "c-cli-selector-status",
            for provider in CliProviderOption::ALL {
                {
                    let is_available = available_providers.contains(&provider.slug().to_string());
                    let is_selected = provider == current;

                    rsx! {
                        button {
                            class: format!(
                                "c-cli-selector-status__btn {} {}",
                                if is_selected { "c-cli-selector-status__btn--selected" } else { "" },
                                if !is_available { "c-cli-selector-status__btn--unavailable" } else { "" }
                            ),
                            disabled: disabled || !is_available,
                            onclick: move |_| {
                                if is_available {
                                    on_change.call(provider);
                                }
                            },
                            title: if is_available {
                                format!("Use {} CLI", provider.display_name())
                            } else {
                                format!("{} CLI not installed", provider.display_name())
                            },
                            span { class: "c-cli-selector-status__icon", "{provider.icon()}" }
                            span { class: "c-cli-selector-status__name", "{provider.display_name()}" }
                            // Image support indicator with tooltip
                            span {
                                class: if provider.supports_images() {
                                    "c-cli-selector-status__badge c-cli-selector-status__badge--supported"
                                } else {
                                    "c-cli-selector-status__badge c-cli-selector-status__badge--unsupported"
                                },
                                title: "{provider.image_support_tooltip()}",
                                "{provider.image_support_display()} 🖼️"
                            }
                            if !is_available {
                                span {
                                    class: "c-cli-selector-status__unavailable",
                                    "Not installed"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
