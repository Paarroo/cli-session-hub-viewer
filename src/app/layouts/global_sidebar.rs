use crate::app::pages::claude_routes::Route;
use dioxus::prelude::*;

#[component]
pub fn GlobalSidebar() -> Element {
    rsx! {
        div { class: "c-sidebar",
            div { class: "c-sidebar__header",
                h2 { "Navigation" }
            }
            nav { class: "c-sidebar__nav",
                ul {
                    li {
                        Link {
                            to: Route::Home {},
                            "🏠 Home"
                        }
                    }
                    li {
                        Link {
                            to: Route::Vision {},
                            "👁️ Vision"
                        }
                    }
                }
            }
        }
    }
}
