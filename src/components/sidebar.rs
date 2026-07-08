use dioxus::prelude::*;

use crate::route::Route;

#[component]
pub fn Sidebar() -> Element {
    rsx! {
        aside { class: "app-sidebar",
            span { class: "sidebar-section", "Main" }
            ul { class: "sidebar-nav",
                li {
                    Link { to: Route::Dashboard {}, "Dashboard" }
                }
                li {
                    Link { to: Route::TimeList {}, "Time" }
                }
                li {
                    Link { to: Route::Timesheet {}, "Timesheet" }
                }
            }
            span { class: "sidebar-section", "Manage" }
            ul { class: "sidebar-nav",
                li {
                    Link { to: Route::ClientList {}, "Clients" }
                }
                li {
                    Link { to: Route::ProjectList {}, "Projects" }
                }
                li {
                    Link { to: Route::Approvals {}, "Approvals" }
                }
                li {
                    Link { to: Route::Reports {}, "Reports" }
                }
                li {
                    Link { to: Route::InvoiceList {}, "Invoices" }
                }
            }
            span { class: "sidebar-section", "System" }
            ul { class: "sidebar-nav",
                li {
                    Link { to: Route::AdminUsers {}, "Users" }
                }
                li {
                    Link { to: Route::Settings {}, "Settings" }
                }
            }
        }
    }
}
