use dioxus::prelude::*;

/// A sidebar rail row: an optional leading icon, a label, and — when `active` —
/// a raised surface, hairline, and trailing pine dot (per the design system).
#[component]
pub fn NavItem(
    label: String,
    #[props(default)] icon: String,
    #[props(default)] active: bool,
    #[props(default)] onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: if active { "nav-item active" } else { "nav-item" },
            "aria-current": if active { "page" },
            onclick: move |e| onclick.call(e),
            if !icon.is_empty() {
                span { class: "nav-item-icon", "{icon}" }
            }
            span { class: "nav-item-label", "{label}" }
            if active {
                span { class: "nav-item-dot" }
            }
        }
    }
}
