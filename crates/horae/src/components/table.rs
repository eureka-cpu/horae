use dioxus::prelude::*;

/// Wrapper for responsive tables
#[component]
pub fn DataTable(children: Element) -> Element {
    rsx! {
        div { class: "table-container",
            {children}
        }
    }
}
