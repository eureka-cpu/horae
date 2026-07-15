use dioxus::prelude::*;

/// A single toast notification. `variant` is `success`, `danger`, `warning`,
/// `info`, or `""` for neutral.
#[component]
pub fn Toast(
    message: String,
    #[props(default)] variant: String,
    #[props(default)] icon: String,
) -> Element {
    rsx! {
        div { class: "toast {variant}", role: "status",
            if !icon.is_empty() {
                span { "{icon}" }
            }
            span { "{message}" }
        }
    }
}

/// The fixed bottom-right stack that holds active toasts.
#[component]
pub fn ToastContainer(children: Element) -> Element {
    rsx! {
        div { class: "toast-container", {children} }
    }
}
