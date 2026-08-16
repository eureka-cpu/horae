use dioxus::prelude::*;

/// The three palettes defined in horae.css (`:root` plus `[data-theme="…"]`
/// overrides). Persisted client-side under `localStorage['horae-theme']` by
/// the script `THEME_SCRIPT` installs — see [`ThemeInit`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
    Pine,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Dark, Theme::Light, Theme::Pine];

    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::Pine => "pine",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Pine => "Pine",
        }
    }

    pub fn from_str(s: &str) -> Theme {
        match s {
            "light" => Theme::Light,
            "pine" => Theme::Pine,
            _ => Theme::Dark,
        }
    }
}

/// Reads the saved theme (or defaults to dark) and applies it before first
/// paint, and defines `setHoraeTheme()` for the settings page to call. Lives
/// in `<head>` via `document::Script` so it runs ahead of body paint — no
/// flash of the wrong theme on reload. Mirrors `site/index.html`.
const THEME_SCRIPT: &str = r#"(function () {
  window.setHoraeTheme = function (t) {
    document.documentElement.dataset.theme = t;
    localStorage.setItem('horae-theme', t);
  };
  document.documentElement.dataset.theme = localStorage.getItem('horae-theme') || 'dark';
})();"#;

#[component]
pub fn ThemeInit() -> Element {
    rsx! {
        document::Script { "{THEME_SCRIPT}" }
    }
}
