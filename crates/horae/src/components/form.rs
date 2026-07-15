use dioxus::prelude::*;

/// A labelled field wrapper with an optional hint line below the control.
#[component]
pub fn FormGroup(label: String, #[props(default)] hint: String, children: Element) -> Element {
    rsx! {
        div { class: "form-group",
            label { class: "form-label", "{label}" }
            {children}
            if !hint.is_empty() {
                p { class: "form-hint", "{hint}" }
            }
        }
    }
}

/// A text input. `kind` maps to the HTML `type` (e.g. `text`, `number`, `email`).
#[component]
pub fn Input(
    #[props(default = "text".to_string())] kind: String,
    #[props(default)] value: String,
    #[props(default)] placeholder: String,
    #[props(default)] disabled: bool,
    #[props(default)] readonly: bool,
    #[props(default)] oninput: EventHandler<FormEvent>,
) -> Element {
    rsx! {
        input {
            class: "form-input",
            r#type: "{kind}",
            value: "{value}",
            placeholder: "{placeholder}",
            disabled,
            readonly,
            oninput: move |e| oninput.call(e),
        }
    }
}

/// A multi-line text input.
#[component]
pub fn Textarea(
    #[props(default)] value: String,
    #[props(default)] placeholder: String,
    #[props(default = 3)] rows: i64,
    #[props(default)] disabled: bool,
    #[props(default)] oninput: EventHandler<FormEvent>,
) -> Element {
    rsx! {
        textarea {
            class: "form-textarea",
            rows: "{rows}",
            placeholder: "{placeholder}",
            disabled,
            oninput: move |e| oninput.call(e),
            "{value}"
        }
    }
}

/// A select built from `(value, label)` option pairs; the option matching
/// `selected` is pre-selected.
#[component]
pub fn Select(
    options: Vec<(String, String)>,
    #[props(default)] selected: String,
    #[props(default)] disabled: bool,
    #[props(default)] onchange: EventHandler<FormEvent>,
) -> Element {
    rsx! {
        select {
            class: "form-select",
            disabled,
            onchange: move |e| onchange.call(e),
            for (value , label) in options {
                option { value: "{value}", selected: value == selected, "{label}" }
            }
        }
    }
}
