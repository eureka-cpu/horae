use dioxus::prelude::*;

use crate::route::Route;

#[component]
pub fn NavBar() -> Element {
    rsx! {
        nav { class: "app-nav",
            Link {
                to: Route::Dashboard {},
                class: "nav-brand",
                "Horae"
            }
            div { class: "nav-spacer" }
            // TODO: user avatar + logout
            Link { to: Route::Settings {}, class: "nav-link text-sm", "Settings" }
        }
    }
}
