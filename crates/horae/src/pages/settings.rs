use dioxus::prelude::*;

#[component]
pub fn Settings() -> Element {
    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Settings" }
            }
            div { class: "card",
                h2 { class: "card-title", "General" }
                p { class: "text-muted text-sm", "Application settings coming soon." }
            }
            div { class: "card mt-4",
                h2 { class: "card-title", "Plugins" }
                p { class: "text-muted text-sm", "No plugins installed. Drop .wasm files into the plugins/ directory." }
            }
        }
    }
}
