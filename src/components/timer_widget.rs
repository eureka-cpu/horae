use dioxus::prelude::*;

use crate::server_fns;

#[component]
pub fn TimerWidget() -> Element {
    let mut running = use_signal(|| false);
    let elapsed = use_signal(|| 0u64);

    let handle_toggle = move |_| {
        spawn(async move {
            if *running.read() {
                // TODO: call stop_timer server fn
                running.set(false);
            } else {
                // TODO: call start_timer server fn
                running.set(true);
            }
        });
    };

    let hours = *elapsed.read() / 3600;
    let minutes = (*elapsed.read() % 3600) / 60;
    let seconds = *elapsed.read() % 60;

    rsx! {
        div { class: if *running.read() { "timer-widget timer-running" } else { "timer-widget" },
            div { class: "timer-display",
                "{hours:02}:{minutes:02}:{seconds:02}"
            }
            div {
                p { class: "text-muted text-sm", "No project selected" }
            }
            div { style: "margin-left: auto;",
                button {
                    class: if *running.read() { "btn btn-danger" } else { "btn btn-primary" },
                    onclick: handle_toggle,
                    if *running.read() { "Stop" } else { "Start" }
                }
            }
        }
    }
}
