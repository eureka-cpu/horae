use dioxus::prelude::*;

/// A panel with an optional title.
#[component]
pub fn Card(#[props(default)] title: String, children: Element) -> Element {
    rsx! {
        div { class: "card",
            if !title.is_empty() {
                h3 { class: "card-title", "{title}" }
            }
            {children}
        }
    }
}

/// A single metric: an uppercase label, a mono value, and an optional signed
/// delta. `direction` is `up`, `down`, or `""` for a neutral delta.
#[component]
pub fn MetricCard(
    label: String,
    value: String,
    #[props(default)] delta: String,
    #[props(default)] direction: String,
) -> Element {
    rsx! {
        div { class: "metric-card",
            span { class: "metric-label", "{label}" }
            span { class: "metric-value", "{value}" }
            if !delta.is_empty() {
                span { class: "metric-delta {direction}", "{delta}" }
            }
        }
    }
}
