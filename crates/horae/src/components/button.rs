use dioxus::prelude::*;

/// A styled action button. `variant` is one of `primary`, `secondary`, `accent`,
/// `danger`, `ghost` (see the design system); `size` is `""` or `sm`.
#[component]
pub fn Button(
    #[props(default = "primary".to_string())] variant: String,
    #[props(default)] size: String,
    #[props(default)] disabled: bool,
    #[props(default)] onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    let mut class = format!("btn btn-{variant}");
    if !size.is_empty() {
        class.push_str(&format!(" btn-{size}"));
    }
    rsx! {
        button {
            class: "{class}",
            disabled,
            onclick: move |e| onclick.call(e),
            {children}
        }
    }
}

/// A square, icon-only button. `label` is required for accessibility (aria-label).
#[component]
pub fn IconButton(
    #[props(default = "ghost".to_string())] variant: String,
    label: String,
    #[props(default)] disabled: bool,
    #[props(default)] onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    rsx! {
        button {
            class: "btn btn-{variant} btn-icon",
            "aria-label": "{label}",
            disabled,
            onclick: move |e| onclick.call(e),
            {children}
        }
    }
}

/// A primary action joined to a secondary trigger (e.g. a dropdown caret).
#[component]
pub fn SplitButton(
    #[props(default = "primary".to_string())] variant: String,
    label: String,
    #[props(default = "▾".to_string())] trigger: String,
    #[props(default)] onclick: EventHandler<MouseEvent>,
    #[props(default)] ontrigger: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "btn-split",
            button { class: "btn btn-{variant}", onclick: move |e| onclick.call(e), "{label}" }
            button {
                class: "btn btn-{variant}",
                "aria-label": "More actions",
                onclick: move |e| ontrigger.call(e),
                "{trigger}"
            }
        }
    }
}
