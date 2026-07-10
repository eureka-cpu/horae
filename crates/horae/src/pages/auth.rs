/// Auth-related page components.
///
/// `/auth/login` and `/auth/logout` are served by Axum routes (see `src/auth/`).
/// This module contains Dioxus components that need session-aware behaviour.
use dioxus::prelude::*;

use crate::server_fns;

/// A logout button that calls the `logout` server function.
/// Full redirect to `/auth/login` happens server-side via the Axum logout route
/// (`POST /auth/logout`). This component is used as a fallback from within the SPA.
#[component]
pub fn LogoutButton() -> Element {
    let mut status = use_signal(String::new);

    let handle_logout = move |_| {
        spawn(async move {
            match server_fns::logout().await {
                Ok(_) => status.set("Signed out — refresh to login.".into()),
                Err(e) => status.set(format!("Logout failed: {e}")),
            }
        });
    };

    rsx! {
        button {
            class: "btn btn-ghost btn-sm",
            onclick: handle_logout,
            "Sign out"
        }
        if !status.read().is_empty() {
            span { class: "text-muted text-sm", " {status}" }
        }
    }
}
