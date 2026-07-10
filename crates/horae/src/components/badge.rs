use dioxus::prelude::*;

#[component]
pub fn Badge(variant: String, children: Element) -> Element {
    let class = format!("badge badge-{}", variant);
    rsx! {
        span { class: "{class}", {children} }
    }
}
