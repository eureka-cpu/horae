use dioxus::prelude::*;

use crate::components::sidebar::Sidebar;

#[component]
pub fn AppLayout() -> Element {
    // Owned here (not in the sidebar) so the shell class can narrow the content
    // area together with the rail when collapsed.
    let collapsed = use_signal(|| false);

    rsx! {
        div { class: if collapsed() { "app-shell collapsed" } else { "app-shell" },
            Sidebar { collapsed }
            main { class: "app-content",
                Outlet::<crate::route::Route> {}
            }
        }
    }
}
