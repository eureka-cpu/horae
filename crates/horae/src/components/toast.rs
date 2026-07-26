use dioxus::prelude::*;

/// A single toast notification. `variant` is `success`, `danger`, `warning`,
/// `info`, or `""` for neutral.
#[component]
pub fn Toast(
    message: String,
    #[props(default)] variant: String,
    #[props(default)] icon: String,
    /// Show a dismiss (×) button that fires `ondismiss`.
    #[props(default)]
    dismissible: bool,
    #[props(default)] ondismiss: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "toast {variant}", role: "status",
            if !icon.is_empty() {
                span { "{icon}" }
            }
            span { class: "toast-msg", "{message}" }
            if dismissible {
                button {
                    r#type: "button",
                    class: "toast-close",
                    "aria-label": "Dismiss",
                    onclick: move |e| ondismiss.call(e),
                    "×"
                }
            }
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
