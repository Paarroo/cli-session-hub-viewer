use dioxus::prelude::*;

/// Reusable delete button with confirmation overlay
/// Uses CSS classes: c-delete-btn, c-delete-btn__icon, c-delete-btn__confirm-overlay, etc.
#[component]
pub fn DeleteButton(
    /// Called when deletion is confirmed
    on_confirm: EventHandler<()>,
    /// Optional: Show loading state
    #[props(default = false)]
    is_loading: bool,
    /// Optional: Custom CSS class for positioning variant
    #[props(default = "")]
    class: &'static str,
    /// Optional: Confirm text
    #[props(default = "Supprimer ?")]
    confirm_text: &'static str,
) -> Element {
    let mut show_confirm = use_signal(|| false);

    let btn_class = if class.is_empty() {
        "c-delete-btn".to_string()
    } else {
        format!("c-delete-btn {}", class)
    };

    rsx! {
        div { class: "c-delete-btn__wrapper",
            // Delete button
            button {
                class: "{btn_class}",
                onclick: move |evt| {
                    evt.stop_propagation();
                    evt.prevent_default();
                    show_confirm.set(true);
                },
                span { class: "c-delete-btn__icon", "🗑️" }
            }

            // Confirmation overlay
            if *show_confirm.read() {
                div { class: "c-delete-btn__confirm-overlay",
                    span { class: "c-delete-btn__confirm-text", "{confirm_text}" }
                    div { class: "c-delete-btn__confirm-actions",
                        button {
                            class: "c-delete-btn__confirm-btn c-delete-btn__confirm-btn--danger",
                            disabled: is_loading,
                            onclick: move |evt| {
                                evt.stop_propagation();
                                on_confirm.call(());
                            },
                            if is_loading { "..." } else { "Oui" }
                        }
                        button {
                            class: "c-delete-btn__confirm-btn c-delete-btn__confirm-btn--cancel",
                            onclick: move |evt| {
                                evt.stop_propagation();
                                show_confirm.set(false);
                            },
                            "Non"
                        }
                    }
                }
            }
        }
    }
}

/// Inline delete button (for session lists) - positioned absolute
#[component]
pub fn InlineDeleteButton(
    on_confirm: EventHandler<()>,
    #[props(default = false)]
    is_loading: bool,
) -> Element {
    let mut show_confirm = use_signal(|| false);

    rsx! {
        // Delete button (always visible, positioned by parent)
        button {
            class: "c-session-item__delete",
            onclick: move |evt| {
                evt.stop_propagation();
                evt.prevent_default();
                show_confirm.set(true);
            },
            "🗑️"
        }

        // Confirmation overlay
        if *show_confirm.read() {
            div { class: "c-session-item__confirm-overlay",
                span { class: "c-session-item__confirm-text", "Supprimer ?" }
                button {
                    class: "c-session-item__confirm-btn c-session-item__confirm-btn--danger",
                    disabled: is_loading,
                    onclick: move |evt| {
                        evt.stop_propagation();
                        on_confirm.call(());
                    },
                    if is_loading { "..." } else { "Oui" }
                }
                button {
                    class: "c-session-item__confirm-btn c-session-item__confirm-btn--cancel",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        show_confirm.set(false);
                    },
                    "Non"
                }
            }
        }
    }
}
