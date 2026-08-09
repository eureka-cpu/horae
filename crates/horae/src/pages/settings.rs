use dioxus::prelude::*;

use crate::components::form::{FormGroup, Select};
use crate::components::theme::Theme;

#[component]
pub fn Settings() -> Element {
    let mut theme = use_signal(|| Theme::Dark);

    // The active theme lives in `<html data-theme>` / localStorage (set by
    // ThemeInit before first paint), not in Rust state, so read it back once
    // the client has mounted to show the dropdown's real current value.
    use_effect(move || {
        spawn(async move {
            if let Ok(current) =
                document::eval("return document.documentElement.dataset.theme || 'dark';")
                    .join::<String>()
                    .await
            {
                theme.set(Theme::from_str(&current));
            }
        });
    });

    let theme_options: Vec<(String, String)> = Theme::ALL
        .iter()
        .map(|t| (t.as_str().to_string(), t.label().to_string()))
        .collect();

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Settings" }
            }
            div { class: "card",
                h2 { class: "card-title", "General" }
                FormGroup { label: "Theme", hint: "Choose how Horae looks on this device.",
                    Select {
                        options: theme_options,
                        selected: theme().as_str().to_string(),
                        onchange: move |e: FormEvent| {
                            let picked = Theme::from_str(&e.value());
                            theme.set(picked);
                            document::eval(&format!("setHoraeTheme('{}')", picked.as_str()));
                        },
                    }
                }
            }
            div { class: "card mt-4",
                h2 { class: "card-title", "Plugins" }
                p { class: "text-muted text-sm", "No plugins installed. Drop .wasm files into the plugins/ directory." }
            }
        }
    }
}
