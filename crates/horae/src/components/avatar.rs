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

/// A compact person/entity chip with a leading avatar.
#[component]
pub fn Chip(label: String, #[props(default)] initials: String) -> Element {
    rsx! {
        span { class: "chip",
            Avatar { initials: if initials.is_empty() { first_initial(&label) } else { initials } }
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
