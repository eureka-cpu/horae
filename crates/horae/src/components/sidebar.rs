use dioxus::prelude::*;

use crate::components::avatar::Avatar;
use crate::components::logo::HoraeMark;
use crate::route::Route;
use crate::server_fns;

/// The left rail: brand, a start-timer action, grouped navigation with an active
/// state, and a footer showing the signed-in user. `collapsed` is owned by
/// `AppLayout` so the shell can narrow the content area in step with the rail.
#[component]
pub fn Sidebar(collapsed: Signal<bool>) -> Element {
    let me = use_resource(|| async move { server_fns::get_me().await });

    rsx! {
        aside { class: "app-sidebar",
            div { class: "sidebar-brand",
                HoraeMark {}
                span { class: "sidebar-brand-name", "Horae" }
                button {
                    class: "sidebar-collapse",
                    title: "Collapse sidebar",
                    "aria-label": "Collapse sidebar",
                    onclick: move |_| collapsed.set(!collapsed()),
                    if collapsed() { "»" } else { "«" }
                }
            }

            Link { to: Route::TimeList {}, class: "sidebar-timer",
                span { class: "sidebar-timer-icon" }
                span { class: "sidebar-timer-label", "Start timer" }
            }

            div { class: "sidebar-section", "Track" }
            div { class: "sidebar-group",
                SideLink { to: Route::Dashboard {}, icon: "◈", label: "Dashboard" }
                SideLink { to: Route::TimeList {}, icon: "◔", label: "Time" }
                SideLink { to: Route::Timesheet {}, icon: "▤", label: "Timesheet" }
            }

            div { class: "sidebar-section", "Organize" }
            div { class: "sidebar-group",
                SideLink { to: Route::ClientList {}, icon: "◇", label: "Clients" }
                SideLink { to: Route::ProjectList {}, icon: "▧", label: "Projects" }
                SideLink { to: Route::InvoiceList {}, icon: "▭", label: "Invoices" }
            }

            div { class: "sidebar-section", "Review" }
            div { class: "sidebar-group",
                SideLink { to: Route::Approvals {}, icon: "✓", label: "Approvals" }
                SideLink { to: Route::Reports {}, icon: "▥", label: "Reports" }
            }

            div { class: "sidebar-section", "System" }
            div { class: "sidebar-group",
                SideLink { to: Route::AdminUsers {}, icon: "◍", label: "Users" }
                SideLink { to: Route::Settings {}, icon: "⚙", label: "Settings" }
            }

            div { class: "sidebar-spacer" }

            div { class: "sidebar-footer",
                match &*me.read() {
                    Some(Ok(u)) => rsx! {
                        Avatar { initials: initials(&u.name) }
                        div { class: "sidebar-user",
                            div { class: "sidebar-user-name truncate", "{u.name}" }
                            div { class: "sidebar-user-sub", "{u.org_role}" }
                        }
                        // A form POST so sign-out works without JS (the /auth/logout
                        // Axum route flushes the session and redirects to login).
                        form { method: "post", action: "/auth/logout",
                            button {
                                class: "sidebar-signout",
                                r#type: "submit",
                                title: "Sign out",
                                "aria-label": "Sign out",
                                "⏻"
                            }
                        }
                    },
                    _ => rsx! {
                        Avatar { initials: "·".to_string() }
                        div { class: "sidebar-user", div { class: "sidebar-user-name", "…" } }
                    },
                }
            }
        }
    }
}

/// One rail row: a client-side `Link` that auto-marks itself active for its route.
#[component]
fn SideLink(to: Route, icon: String, label: String) -> Element {
    rsx! {
        Link { to, active_class: "active", class: "nav-item",
            span { class: "nav-item-icon", "{icon}" }
            span { class: "nav-item-label", "{label}" }
        }
    }
}

/// Up to two leading initials from a display name, uppercased.
fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}
