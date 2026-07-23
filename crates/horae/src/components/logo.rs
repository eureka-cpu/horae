use dioxus::prelude::*;

#[component]
pub fn HoraeMark() -> Element {
    rsx! {
        img {
            class: "horae-mark",
            src: asset!("/assets/horae-icon.svg"),
            alt: "Horae",
        }
    }
}
