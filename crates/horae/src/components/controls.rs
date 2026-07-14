use dioxus::prelude::*;

/// A segmented control: one active item out of several. Emits the chosen label.
#[component]
pub fn Segmented(
    items: Vec<String>,
    active: String,
    #[props(default)] onselect: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "segmented",
            for item in items {
                button {
                    class: if item == active { "segmented-item active" } else { "segmented-item" },
                    onclick: {
                        let item = item.clone();
                        move |_| onselect.call(item.clone())
                    },
                    "{item}"
                }
            }
        }
    }
}

/// An on/off switch with a trailing label.
#[component]
pub fn Toggle(
    on: bool,
    #[props(default)] label: String,
    #[props(default)] onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div {
            class: if on { "toggle on" } else { "toggle" },
            role: "switch",
            "aria-checked": "{on}",
            onclick: move |e| onclick.call(e),
            span { class: "toggle-track", span { class: "toggle-thumb" } }
            if !label.is_empty() {
                span { "{label}" }
            }
        }
    }
}

/// A checkbox with a label. Controlled via `checked`.
#[component]
pub fn Checkbox(
    checked: bool,
    label: String,
    #[props(default)] onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div {
            class: if checked { "choice checked" } else { "choice" },
            role: "checkbox",
            "aria-checked": "{checked}",
            onclick: move |e| onclick.call(e),
            span { class: "choice-box checkbox", if checked { "✓" } }
            span { "{label}" }
        }
    }
}

/// A radio option with a label. Controlled via `selected`.
#[component]
pub fn Radio(
    selected: bool,
    label: String,
    #[props(default)] onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div {
            class: if selected { "choice checked" } else { "choice" },
            role: "radio",
            "aria-checked": "{selected}",
            onclick: move |e| onclick.call(e),
            span { class: "choice-box radio" }
            span { "{label}" }
        }
    }
}
