use dioxus::prelude::*;

use crate::route::Route;

#[allow(non_snake_case)]
pub fn App() -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: asset!("/assets/css/horae.css"),
        }
        // Generated utility layer (built by crates/horae/build.rs); tokens/components live in horae.css.
        document::Link {
            rel: "stylesheet",
            href: asset!("/assets/css/horae-utils.css"),
        }
        Router::<Route> {}
    }
}
