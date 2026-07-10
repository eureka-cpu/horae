use dioxus::prelude::*;

use crate::components::nav::NavBar;
use crate::components::sidebar::Sidebar;

#[component]
pub fn AppLayout() -> Element {
    rsx! {
        div { class: "app-layout",
            NavBar {}
            div { class: "app-body",
                Sidebar {}
                main { class: "app-content",
                    Outlet::<crate::route::Route> {}
                }
            }
        }
    }
}
