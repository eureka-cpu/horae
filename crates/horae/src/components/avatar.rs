use dioxus::prelude::*;

/// A circular avatar. Shows `image_url` when given, otherwise the `initials`.
/// `size` is `sm`, `""` (default), or `lg`.
#[component]
pub fn Avatar(
    initials: String,
    #[props(default)] image_url: String,
    #[props(default)] size: String,
) -> Element {
    let class = if size.is_empty() {
        "avatar".to_string()
    } else {
        format!("avatar avatar-{size}")
    };
    rsx! {
        span { class: "{class}", "aria-label": "{initials}",
            if image_url.is_empty() {
                "{initials}"
            } else {
                img { src: "{image_url}", alt: "{initials}" }
            }
        }
    }
}

/// A compact chip. By default it leads with an avatar (a person chip); set
/// `plain` for an avatar-less tag chip (e.g. "Time & Materials").
#[component]
pub fn Chip(
    label: String,
    #[props(default)] initials: String,
    #[props(default)] plain: bool,
) -> Element {
    rsx! {
        span { class: if plain { "chip chip-plain" } else { "chip" },
            if !plain {
                Avatar { initials: if initials.is_empty() { first_initial(&label) } else { initials } }
            }
            "{label}"
        }
    }
}

/// The first character of `name`, uppercased — a default avatar fallback.
fn first_initial(name: &str) -> String {
    name.chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default()
}
