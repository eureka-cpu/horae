use dioxus::prelude::*;

use crate::route::Route;

#[allow(non_snake_case)]
pub fn App() -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: asset!("/assets/css/horae.css"),
        }
        Router::<Route> {}
    }
}
