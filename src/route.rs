use dioxus::prelude::*;
use uuid::Uuid;

use crate::pages::{
    auth::{Login, Register},
    admin::AdminUsers,
    clients::{ClientDetail, ClientList},
    dashboard::Dashboard,
    invoices::{InvoiceDetail, InvoiceList},
    projects::{ProjectDetail, ProjectList},
    settings::Settings,
    time::TimeList,
};
use crate::components::layout::AppLayout;

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
                        to: Route::Dashboard {},
                        class: "btn btn-primary",
                        "Go to Dashboard"
                    }
                }
            }
        }
    }
}

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[route("/auth/login")]
    Login {},
    #[route("/auth/register")]
    Register {},
    #[layout(AppLayout)]
    #[route("/")]
    Dashboard {},
    #[route("/clients")]
    ClientList {},
    #[route("/clients/:id")]
    ClientDetail { id: Uuid },
    #[route("/projects")]
    ProjectList {},
    #[route("/projects/:id")]
    ProjectDetail { id: Uuid },
    #[route("/time")]
    TimeList {},
    #[route("/invoices")]
    InvoiceList {},
    #[route("/invoices/:id")]
    InvoiceDetail { id: Uuid },
    #[route("/admin/users")]
    AdminUsers {},
    #[route("/settings")]
    Settings {},
    #[route("/:..route")]
    NotFound { route: Vec<String> },
}
