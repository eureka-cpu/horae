use dioxus::prelude::*;

/// The Horae mark: a rising sun over pine, inlined so its fills track the
/// `--icon-*` tokens and re-theme with the rest of the app (see horae.css).
#[component]
pub fn HoraeMark() -> Element {
    rsx! {
        svg {
            class: "horae-mark",
            view_box: "0 0 128 128",
            "aria-hidden": "true",
            defs {
                clipPath { id: "horae-mark-clip",
                    rect { width: "128", height: "128", rx: "30" }
                }
            }
            rect { width: "128", height: "128", rx: "30", style: "fill: var(--icon-bg-fill)" }
            line { x1: "64", y1: "70", x2: "64", y2: "24", style: "stroke: var(--icon-ray-center)", stroke_width: "2.5", stroke_linecap: "round" }
            line { x1: "64", y1: "70", x2: "57", y2: "35", style: "stroke: var(--icon-ray-side)", stroke_width: "2.5", stroke_linecap: "round" }
            line { x1: "64", y1: "70", x2: "71", y2: "35", style: "stroke: var(--icon-ray-side)", stroke_width: "2.5", stroke_linecap: "round" }
            circle { cx: "64", cy: "70", r: "29", style: "fill: var(--icon-sun-fill)" }
            rect { y: "85", width: "128", height: "2", style: "fill: var(--icon-horizon-line)", clip_path: "url(#horae-mark-clip)" }
            rect { y: "87", width: "128", height: "41", style: "fill: var(--icon-horizon-fill)", clip_path: "url(#horae-mark-clip)" }
        }
    }
}
