use dioxus::prelude::*;

/// A circular avatar. Shows `image_url` when given, otherwise the `initials`.
/// `size` is `sm`, `""` (default), or `lg`. `empty` renders the muted,
/// dashed-border placeholder (an unassigned slot).
#[component]
pub fn Avatar(
    initials: String,
    #[props(default)] image_url: String,
    #[props(default)] size: String,
    #[props(default)] empty: bool,
) -> Element {
    let mut class = String::from("avatar");
    if !size.is_empty() {
        class.push_str(&format!(" avatar-{size}"));
    }
    if empty {
        class.push_str(" avatar-empty");
    }
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
/// `plain` for an avatar-less tag chip (e.g. "Time & Materials"). `variant`
/// (`success`, `warning`, `danger`, `info`) tints it like a badge.
#[component]
pub fn Chip(
    label: String,
    #[props(default)] initials: String,
    #[props(default)] plain: bool,
    #[props(default)] variant: String,
) -> Element {
    let mut class = String::from("chip");
    if plain {
        class.push_str(" chip-plain");
    }
    if !variant.is_empty() {
        class.push_str(&format!(" chip-{variant}"));
    }
    rsx! {
        span { class: "{class}",
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
