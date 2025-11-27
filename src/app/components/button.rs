use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
}

#[component]
pub fn Button(
    variant: Option<ButtonVariant>,
    disabled: Option<bool>,
    onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let variant = variant.unwrap_or(ButtonVariant::Primary);
    let disabled = disabled.unwrap_or(false);

    let variant_class = match variant {
        ButtonVariant::Primary => "c-button--primary",
        ButtonVariant::Secondary => "c-button--secondary",
        ButtonVariant::Danger => "c-button--danger",
    };

    rsx! {
        button {
            class: "c-button {variant_class}",
            disabled: disabled,
            onclick: move |evt| {
                if let Some(handler) = &onclick {
                    handler.call(evt);
                }
            },
            {children}
        }
    }
}
