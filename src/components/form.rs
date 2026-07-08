use dioxus::prelude::*;

#[component]
pub fn FormGroup(label: String, children: Element) -> Element {
    rsx! {
        div { class: "form-group",
            label { class: "form-label", "{label}" }
            {children}
        }
    }
}
