use dioxus::prelude::*;

/// The Horae mark: a rising sun over a horizon, ringed by hour-ticks (a sundial
/// at dawn). Built from layered elements inside a rounded, clipped pine tile so
/// it scales cleanly; size it via the `.horae-mark` box in CSS.
#[component]
pub fn HoraeMark() -> Element {
    rsx! {
        span { class: "horae-mark", "aria-hidden": "true",
            span { class: "ray ray-l" }
            span { class: "ray ray-c" }
            span { class: "ray ray-r" }
            span { class: "sun" }
            span { class: "horizon-line" }
            span { class: "horizon" }
        }
    }
}
