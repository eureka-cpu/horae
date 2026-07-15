use dioxus::prelude::*;

/// A 16×16 stroked navigation glyph, drawn in `currentColor` so it inherits the
/// rail row's muted colour. Unknown names render nothing.
#[component]
pub fn NavIcon(name: String) -> Element {
    let body = match name.as_str() {
        "dashboard" => rsx! {
            rect { x: "2", y: "2", width: "5", height: "5", rx: "1" }
            rect { x: "9", y: "2", width: "5", height: "5", rx: "1" }
            rect { x: "2", y: "9", width: "5", height: "5", rx: "1" }
            rect { x: "9", y: "9", width: "5", height: "5", rx: "1" }
        },
        "time" => rsx! {
            circle { cx: "8", cy: "8", r: "6" }
            path { d: "M8 4.5 V8 L10.5 9.5" }
        },
        "timesheet" => rsx! {
            rect { x: "2.5", y: "3", width: "11", height: "10.5", rx: "1.5" }
            line { x1: "2.5", y1: "6", x2: "13.5", y2: "6" }
            line { x1: "5.2", y1: "1.6", x2: "5.2", y2: "4" }
            line { x1: "10.8", y1: "1.6", x2: "10.8", y2: "4" }
        },
        "clients" => rsx! {
            rect { x: "2", y: "5", width: "12", height: "8.5", rx: "1.5" }
            path { d: "M6 5 V3.6 A1 1 0 0 1 7 2.6 H9 A1 1 0 0 1 10 3.6 V5" }
        },
        "projects" => rsx! {
            path { d: "M2 4.6 A1 1 0 0 1 3 3.6 H6 L7.6 5.2 H13 A1 1 0 0 1 14 6.2 V12 A1 1 0 0 1 13 13 H3 A1 1 0 0 1 2 12 Z" }
        },
        "invoices" => rsx! {
            rect { x: "3.5", y: "2", width: "9", height: "12", rx: "1" }
            line { x1: "5.8", y1: "5.5", x2: "10.2", y2: "5.5" }
            line { x1: "5.8", y1: "8", x2: "10.2", y2: "8" }
            line { x1: "5.8", y1: "10.5", x2: "8.5", y2: "10.5" }
        },
        "approvals" => rsx! {
            circle { cx: "8", cy: "8", r: "6" }
            path { d: "M5.4 8 L7 9.6 L10.6 6" }
        },
        "reports" => rsx! {
            line { x1: "4", y1: "13", x2: "4", y2: "9" }
            line { x1: "8", y1: "13", x2: "8", y2: "5" }
            line { x1: "12", y1: "13", x2: "12", y2: "7" }
        },
        "users" => rsx! {
            circle { cx: "8", cy: "5.5", r: "2.5" }
            path { d: "M3.6 13 A4.4 4.4 0 0 1 12.4 13" }
        },
        "settings" => rsx! {
            line { x1: "3", y1: "5", x2: "13", y2: "5" }
            circle { cx: "6", cy: "5", r: "1.6" }
            line { x1: "3", y1: "11", x2: "13", y2: "11" }
            circle { cx: "10", cy: "11", r: "1.6" }
        },
        _ => rsx! {},
    };
    rsx! {
        svg {
            width: "15",
            height: "15",
            view_box: "0 0 16 16",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.4",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            {body}
        }
    }
}
