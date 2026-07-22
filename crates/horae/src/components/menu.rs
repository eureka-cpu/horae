use dioxus::prelude::*;

/// A dropdown menu: a trigger button that reveals a popover of [`MenuItem`]s.
/// Manages its own open state and closes on an outside click or when any item is
/// chosen. Box and item styling come from the shared `.menu` / `.menu-item`
/// classes (also used by the sidebar account menu).
#[component]
pub fn Menu(
    /// Text shown on the trigger button (a `▾` caret is appended).
    label: String,
    /// Anchor the popover to the right edge — for right-aligned cells.
    #[props(default)]
    align_right: bool,
    children: Element,
) -> Element {
    let mut open = use_signal(|| false);
    let popover = if align_right {
        "menu menu-pop menu-pop-right"
    } else {
        "menu menu-pop"
    };
    rsx! {
        div { class: "menu-anchor",
            button {
                r#type: "button",
                class: "btn btn-secondary btn-sm",
                "aria-haspopup": "menu",
                "aria-expanded": "{open}",
                onclick: move |_| {
                    let next = !open();
                    open.set(next);
                },
                "{label}"
                span { class: "ml-2 text-faint", "▾" }
            }
            if open() {
                div { class: "menu-overlay", onclick: move |_| open.set(false) }
                div {
                    class: "{popover}",
                    role: "menu",
                    // Any click inside picks an item; close once it bubbles here.
                    onclick: move |_| open.set(false),
                    {children}
                }
            }
        }
    }
}

/// One selectable row inside a [`Menu`].
#[component]
pub fn MenuItem(
    #[props(default)] onclick: EventHandler<MouseEvent>,
    #[props(default)] selected: bool,
    #[props(default)] danger: bool,
    #[props(default)] disabled: bool,
    children: Element,
) -> Element {
    let mut class = String::from("menu-item");
    if selected {
        class.push_str(" selected");
    }
    if danger {
        class.push_str(" danger");
    }
    if disabled {
        class.push_str(" disabled");
    }
    rsx! {
        button {
            r#type: "button",
            class: "{class}",
            disabled,
            onclick: move |e| onclick.call(e),
            {children}
        }
    }
}

/// A hairline separator between groups of [`MenuItem`]s.
#[component]
pub fn MenuDivider() -> Element {
    rsx! {
        div { class: "menu-divider" }
    }
}
