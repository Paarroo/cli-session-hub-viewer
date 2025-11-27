use dioxus::prelude::*;
use crate::shared::hooks::{use_theme, Theme, save_theme};

/// Theme toggle component for switching between light and dark theme variants.
/// Features animated sun/moon with clouds and stars.
/// Uses the unified theme system with intelligent light/dark toggling.
#[component]
pub fn ThemeToggle() -> Element {
    // Use the unified theme hook
    let (mut current_theme, _default_dark, _default_light) = use_theme();

    // Determine if current theme is light or dark
    let is_currently_light = !current_theme().is_dark();

    let toggle_theme = move |_| {
        // Toggle between light and dark variants
        let new_theme = current_theme().toggle_light_dark();

        // Update the current theme
        current_theme.set(new_theme.clone());

        spawn(async move {
            // Apply the theme CSS
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
                "#, new_theme.as_str());

                let _ = document::eval(&script).await;
            }

            // Save to localStorage
            save_theme(new_theme).await;
        });
    };

    // Tooltip shows target state (what will happen on click)
    let target_theme = current_theme().toggle_light_dark();
    let tooltip = format!("Passer au thème {}", target_theme.display_name());

    let toggle_class = if is_currently_light {
        "c-theme-toggle c-theme-toggle--light"
    } else {
        "c-theme-toggle"
    };

    rsx! {
        div {
            class: "{toggle_class}",
            "data-tooltip": "{tooltip}",
            role: "button",
            tabindex: "0",
            aria_label: "Basculer le mode jour/nuit",
            onclick: toggle_theme,

            // Ball (sun/moon)
            div { class: "c-theme-toggle__ball" }

            // Stars (visible in dark mode)
            div { class: "c-theme-toggle__stars",
                span { class: "c-theme-toggle__star" }
                span { class: "c-theme-toggle__star" }
                span { class: "c-theme-toggle__star" }
                span { class: "c-theme-toggle__star" }
                span { class: "c-theme-toggle__star" }
            }

            // Clouds (visible in light mode)
            div { class: "c-theme-toggle__clouds",
                span { class: "c-theme-toggle__cloud" }
                span { class: "c-theme-toggle__cloud" }
                span { class: "c-theme-toggle__cloud" }
            }
        }
    }
}


