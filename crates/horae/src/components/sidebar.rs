use dioxus::prelude::*;

use crate::components::avatar::Avatar;
use crate::components::icons::NavIcon;
use crate::components::logo::HoraeMark;
use crate::route::Route;
use crate::server_fns;

/// The left rail: brand, a start-timer action, grouped navigation with an active
/// state, and a footer showing the signed-in user. `collapsed` is owned by
/// `AppLayout` so the shell can narrow the content area in step with the rail.
#[component]
pub fn Sidebar(collapsed: Signal<bool>) -> Element {
    rsx! {
        aside { class: "app-sidebar",
            div { class: "sidebar-brand",
                HoraeMark {}
                span { class: "sidebar-brand-name", "Horae" }
                span { class: "sidebar-brand-dot" }
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
                SideLink { to: Route::Dashboard {}, icon: "dashboard", label: "Dashboard" }
                SideLink { to: Route::TimeList {}, icon: "time", label: "Time" }
                SideLink { to: Route::Timesheet {}, icon: "timesheet", label: "Timesheet" }
            }

            div { class: "sidebar-section", "Organize" }
            div { class: "sidebar-group",
                SideLink { to: Route::ClientList {}, icon: "clients", label: "Clients" }
                SideLink { to: Route::ProjectList {}, icon: "projects", label: "Projects" }
                SideLink { to: Route::InvoiceList {}, icon: "invoices", label: "Invoices" }
            }

            div { class: "sidebar-section", "Review" }
            div { class: "sidebar-group",
                SideLink { to: Route::Approvals {}, icon: "approvals", label: "Approvals" }
                SideLink { to: Route::Reports {}, icon: "reports", label: "Reports" }
            }

            div { class: "sidebar-section", "System" }
            div { class: "sidebar-group",
                SideLink { to: Route::AdminUsers {}, icon: "users", label: "Users" }
                SideLink { to: Route::Settings {}, icon: "settings", label: "Settings" }
            }

            div { class: "sidebar-spacer" }

            SidebarUser {}
        }
    }
}

/// One rail row: a client-side `Link` that auto-marks itself active for its route.
/// The glyph is shown when inactive; the active route swaps it for a pine dot
/// (via `.nav-item.active` CSS), matching the design's rail language.
#[component]
fn SideLink(to: Route, icon: String, label: String) -> Element {
    rsx! {
        Link { to, active_class: "active", class: "nav-item",
            span { class: "nav-item-icon", NavIcon { name: icon } }
            span { class: "nav-item-dot" }
            span { class: "nav-item-label", "{label}" }
        }
    }
}

/// The signed-in user: an avatar + name + role row that opens an account popover
/// (profile, notifications, sign out). Falls back to a placeholder until `get_me`
/// resolves (or when not authenticated).
#[component]
fn SidebarUser() -> Element {
    let me = use_resource(|| async move { server_fns::get_me().await });
    let mut open = use_signal(|| false);

    let user = me.read();
    let (name, role, marks) = match &*user {
        Some(Ok(u)) => (u.name.clone(), u.org_role.to_string(), initials(&u.name)),
        _ => ("Not signed in".to_string(), String::new(), "·".to_string()),
    };

    rsx! {
        div { class: "sidebar-userbox",
            if open() {
                div { class: "sidebar-menu menu",
                    div { class: "sidebar-menu-head",
                        Avatar { initials: "{marks}" }
                        div { class: "sidebar-user",
                            div { class: "sidebar-user-name truncate", "{name}" }
                            if !role.is_empty() {
                                div { class: "sidebar-user-sub", "{role}" }
                            }
                        }
                    }
                    div { class: "sidebar-menu-list",
                        Link { to: Route::Settings {}, class: "menu-item", onclick: move |_| open.set(false), "My profile" }
                        Link { to: Route::Settings {}, class: "menu-item", onclick: move |_| open.set(false), "Notifications" }
                    }
                    div { class: "sidebar-menu-foot",
                        form { method: "post", action: "/auth/logout",
                            button { class: "menu-item danger", r#type: "submit", "Sign out" }
                        }
                    }
                }
            }

            button {
                class: "sidebar-footer",
                "aria-haspopup": "menu",
                "aria-expanded": "{open()}",
                onclick: move |_| open.set(!open()),
                Avatar { initials: "{marks}" }
                div { class: "sidebar-user",
                    div { class: "sidebar-user-name truncate", "{name}" }
                    if !role.is_empty() {
                        div { class: "sidebar-user-sub", "{role}" }
                    }
                }
                span { class: "sidebar-user-caret", "⌄" }
            }
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
