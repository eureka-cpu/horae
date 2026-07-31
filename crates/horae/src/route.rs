use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::layout::AppLayout;
use crate::pages::{
    admin::AdminUsers,
    approvals::Approvals,
    clients::{ClientDetail, ClientList},
    gallery::Gallery,
    invoices::{InvoiceDetail, InvoiceList},
    projects::{ProjectDetail, ProjectList},
    reports::Reports,
    settings::Settings,
    timesheet::Timesheet,
};

#[component]
fn NotFound(route: Vec<String>) -> Element {
    rsx! {
        div { class: "auth-container",
            div { class: "auth-card",
                h1 { style: "font-size: 2rem; color: var(--color-text-muted); text-align: center;", "404" }
                p { style: "text-align: center; color: var(--color-text-secondary);",
                    "Page not found: /{route.join(\"/\")}"
                }
                div { style: "text-align: center; margin-top: 1rem;",
                    Link {
                        to: Route::Timesheet {},
                        class: "btn btn-primary",
                        "Go to Timesheet"
                    }
                }
            }
        }
    }
}

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    // /auth/* routes are handled by Axum directly (see src/auth/mod.rs).
    // The Dioxus router only manages the authenticated SPA.
    #[layout(AppLayout)]
    #[route("/")]
    Timesheet {},
    #[route("/clients")]
    ClientList {},
    #[route("/clients/:id")]
    ClientDetail { id: Uuid },
    #[route("/projects")]
    ProjectList {},
    #[route("/projects/:id")]
    ProjectDetail { id: Uuid },
    #[route("/approvals")]
    Approvals {},
    #[route("/reports")]
    Reports {},
    #[route("/invoices")]
    InvoiceList {},
    #[route("/invoices/:id")]
    InvoiceDetail { id: Uuid },
    #[route("/admin/users")]
    AdminUsers {},
    #[route("/settings")]
    Settings {},
    #[route("/components")]
    Gallery {},
    #[end_layout]
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}
