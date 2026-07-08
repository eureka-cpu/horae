use dioxus::prelude::*;

use crate::route::Route;

pub fn App() -> Element {
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: asset!("/assets/css/horae.css"),
        }
        Router::<Route> {}
    }
}
