use dioxus::prelude::*;
use crate::shared::hooks::{use_theme, Theme, save_theme, save_default_theme};

/// Theme selector panel component
#[component]
pub fn ThemeSelector(is_open: Signal<bool>) -> Element {
    // Use the unified theme hook
    let (mut current_theme, mut default_dark, mut default_light) = use_theme();

    // Helper function to select theme - defined as a regular function to avoid mutable closure issues
    fn do_select_theme(
        theme: Theme,
        mut current_theme: Signal<Theme>,
        mut is_open: Signal<bool>,
    ) {
        current_theme.set(theme.clone());

        spawn(async move {
            // Apply theme CSS
            #[cfg(target_arch = "wasm32")]
            {
                let script = format!(r#"
                    (function() {{
                        const root = document.documentElement;
                        const classes = ['dark', 'light', 'golden', 'pistachio', 'dark-golden', 'dark-pistachio'];

                        // Remove all theme classes
                        classes.forEach(cls => root.classList.remove(cls));

                        // Add new theme class
                        root.classList.add('{}');
                    }})()
                "#, theme.as_str());

                let _ = document::eval(&script).await;
            }

            // Save to localStorage
            save_theme(theme).await;
        });

        is_open.set(false);
    }

    let set_as_default = move |theme: Theme| {
        spawn(async move {
            if theme.is_dark() {
                default_dark.set(theme.clone());
                save_default_theme(theme, true).await;
            } else {
                default_light.set(theme.clone());
                save_default_theme(theme, false).await;
            }
        });
    };

    if !is_open() {
        return rsx! {};
    }

    rsx! {
        // Backdrop
        div {
            class: "c-theme-selector__backdrop",
            onclick: move |_| is_open.set(false),
        }

        // Panel
        div { class: "c-theme-selector",
            div { class: "c-theme-selector__header",
                h3 { class: "c-theme-selector__title", "Thème" }
                button {
                    class: "c-theme-selector__close",
                    onclick: move |_| is_open.set(false),
                    "✕"
                }
            }

            div { class: "c-theme-selector__options",
                // Dark themes group
                div { class: "c-theme-selector__group-label", "🌙 Sombres" }
                { [Theme::Dark, Theme::DarkGolden, Theme::DarkPistachio].iter().map(|theme_item| {
                    rsx! {
                        button {
                            class: if current_theme() == *theme_item { "c-theme-selector__option is-active" } else { "c-theme-selector__option" },
                            onclick: move |_| do_select_theme(theme_item.clone(), current_theme.clone(), is_open.clone()),
                            span { class: "c-theme-selector__option-icon", "{theme_item.icon()}" }
                            span { class: "c-theme-selector__option-name", "{theme_item.display_name()}" }
                            if default_dark() == *theme_item {
                                span { class: "c-theme-selector__option-default", "Par défaut" }
                            }
                            if current_theme() == *theme_item {
                                span { class: "c-theme-selector__option-check", "✓" }
                            }
                            // Set as default button
                            if default_dark() != *theme_item {
                                button {
                                    class: "c-theme-selector__set-default",
                                    onclick: move |e| {
                                        e.stop_propagation();
                                        set_as_default(theme_item.clone());
                                    },
                                    title: "Définir par défaut",
                                    "★"
                                }
                            }
                        }
                    }
                })}

                // Separator
                div { class: "c-theme-selector__separator" }

                // Light themes group
                div { class: "c-theme-selector__group-label", "☀️ Clairs" }
                { [Theme::Light, Theme::Golden, Theme::Pistachio].iter().map(|theme_item| {
                    rsx! {
                        button {
                            class: if current_theme() == *theme_item { "c-theme-selector__option is-active" } else { "c-theme-selector__option" },
                            onclick: move |_| do_select_theme(theme_item.clone(), current_theme.clone(), is_open.clone()),
                            span { class: "c-theme-selector__option-icon", "{theme_item.icon()}" }
                            span { class: "c-theme-selector__option-name", "{theme_item.display_name()}" }
                            if default_light() == *theme_item {
                                span { class: "c-theme-selector__option-default", "Par défaut" }
                            }
                            if current_theme() == *theme_item {
                                span { class: "c-theme-selector__option-check", "✓" }
                            }
                            // Set as default button
                            if default_light() != *theme_item {
                                button {
                                    class: "c-theme-selector__set-default",
                                    onclick: move |e| {
                                        e.stop_propagation();
                                        set_as_default(theme_item.clone());
                                    },
                                    title: "Définir par défaut",
                                    "★"
                                }
                            }
                        }
                    }
                })}
            }
        }
    }
}





/// Settings button for sidebar footer
#[component]
pub fn SettingsButton(on_click: EventHandler<()>) -> Element {
    rsx! {
        button {
            class: "c-settings-btn",
            onclick: move |_| on_click.call(()),
            title: "Paramètres",
            span { class: "c-settings-btn__icon", "⚙️" }
            span { class: "c-settings-btn__text", "Paramètres" }
        }
    }
}
