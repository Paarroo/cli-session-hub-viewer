use dioxus::prelude::*;
use std::str::FromStr;

/// Available themes - unified enum for all theme components
#[derive(Clone, Debug, PartialEq)]
pub enum Theme {
    Dark,
    DarkGolden,
    DarkPistachio,
    Light,
    Golden,
    Pistachio,
}

impl Theme {
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::Golden => "golden",
            Theme::Pistachio => "pistachio",
            Theme::DarkGolden => "dark-golden",
            Theme::DarkPistachio => "dark-pistachio",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Theme::Dark => "Sombre",
            Theme::Light => "Clair",
            Theme::Golden => "Heure Dorée",
            Theme::Pistachio => "Pistache",
            Theme::DarkGolden => "Sombre Doré",
            Theme::DarkPistachio => "Sombre Pistache",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Theme::Dark => "🌙",
            Theme::Light => "☀️",
            Theme::Golden => "🌅",
            Theme::Pistachio => "🌿",
            Theme::DarkGolden => "🌑",
            Theme::DarkPistachio => "🌲",
        }
    }

    pub fn is_dark(&self) -> bool {
        matches!(self, Theme::Dark | Theme::DarkGolden | Theme::DarkPistachio)
    }

    pub fn dark_themes() -> [Theme; 3] {
        [Theme::Dark, Theme::DarkGolden, Theme::DarkPistachio]
    }

    pub fn light_themes() -> [Theme; 3] {
        [Theme::Light, Theme::Golden, Theme::Pistachio]
    }

    /// Get the appropriate default theme based on system preference
    pub fn system_default(is_dark_preferred: bool) -> Theme {
        if is_dark_preferred {
            Theme::Dark
        } else {
            Theme::Light
        }
    }

    /// Toggle between light and dark theme variants
    pub fn toggle_light_dark(&self) -> Theme {
        match self {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Dark,
            Theme::Golden => Theme::DarkGolden,
            Theme::DarkGolden => Theme::Golden,
            Theme::Pistachio => Theme::DarkPistachio,
            Theme::DarkPistachio => Theme::Pistachio,
        }
    }
}

impl FromStr for Theme {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "light" => Ok(Theme::Light),
            "golden" => Ok(Theme::Golden),
            "pistachio" => Ok(Theme::Pistachio),
            "dark-golden" => Ok(Theme::DarkGolden),
            "dark-pistachio" => Ok(Theme::DarkPistachio),
            "dark" => Ok(Theme::Dark),
            _ => Ok(Theme::Dark), // Default to dark
        }
    }
}

/// Unified theme hook that manages theme state and persistence
pub fn use_theme() -> (Signal<Theme>, Signal<Theme>, Signal<Theme>) {
    let mut current_theme = use_signal(|| Theme::Dark);
    let mut default_dark = use_signal(|| Theme::Dark);
    let mut default_light = use_signal(|| Theme::Light);

    // Initialize theme from localStorage on mount
    use_effect(move || {
        spawn(async move {
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(window) = web_sys::window() {
                    if let Ok(Some(storage)) = window.local_storage() {
                        // Load current theme
                        if let Ok(Some(saved)) = storage.get_item("theme") {
                            if let Ok(theme) = saved.parse::<Theme>() {
                                let theme_clone = theme.clone();
                                current_theme.set(theme);
                                apply_theme_css(theme_clone).await;
                            }
                        }

                        // Load default preferences
                        if let Ok(Some(saved)) = storage.get_item("default_dark") {
                            if let Ok(theme) = saved.parse::<Theme>() {
                                default_dark.set(theme);
                            }
                        }
                        if let Ok(Some(saved)) = storage.get_item("default_light") {
                            if let Ok(theme) = saved.parse::<Theme>() {
                                default_light.set(theme);
                            }
                        }
                    }
                }
            }

            // If no theme was loaded, detect system preference
            if current_theme() == Theme::Dark && default_dark() == Theme::Dark && default_light() == Theme::Light {
                #[cfg(target_arch = "wasm32")]
                {
                    let script = r#"
                        window.matchMedia('(prefers-color-scheme: dark)').matches
                    "#;
                    if let Ok(result) = document::eval(script).await {
                        if let Some(is_dark) = result.as_bool() {
                            let system_theme = Theme::system_default(is_dark);
                            let system_theme_clone = system_theme.clone();
                            current_theme.set(system_theme);
                            apply_theme_css(system_theme_clone).await;
                        }
                    }
                }
            }
        });
    });

    (current_theme, default_dark, default_light)
}

/// Apply theme CSS classes to document element
#[cfg(target_arch = "wasm32")]
async fn apply_theme_css(theme: Theme) {
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

#[cfg(not(target_arch = "wasm32"))]
async fn apply_theme_css(_theme: Theme) {
    // No-op on server
}

/// Save theme to localStorage
#[cfg(target_arch = "wasm32")]
pub async fn save_theme(theme: Theme) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item("theme", theme.as_str());
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn save_theme(_theme: Theme) {
    // No-op on server
}

/// Save default theme preference
#[cfg(target_arch = "wasm32")]
pub async fn save_default_theme(theme: Theme, is_dark: bool) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let key = if is_dark { "default_dark" } else { "default_light" };
            let _ = storage.set_item(key, theme.as_str());
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn save_default_theme(_theme: Theme, _is_dark: bool) {
    // No-op on server
}