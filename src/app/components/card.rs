use dioxus::prelude::*;

#[component]
pub fn Card(
    title: Option<String>,
    featured: Option<bool>,
    children: Element,
) -> Element {
    let featured = featured.unwrap_or(false);
    let featured_class = if featured { "c-card--featured" } else { "" };

    rsx! {
        div {
            class: "c-card {featured_class}",
            if let Some(title) = title {
                div {
                    class: "c-card__header",
                    h3 {
                        class: "c-card__title",
                        "{title}"
                    }
                }
            }
            div {
                class: "c-card__body",
                {children}
            }
        }
    }
}
