use dioxus::prelude::*;
use uuid::Uuid;

use crate::server_fns;

#[component]
pub fn ProjectList() -> Element {
    let projects = use_resource(|| async move { server_fns::list_projects(None, None).await });

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Projects" }
                div { class: "page-actions",
                    button { class: "btn btn-primary", "New Project" }
                }
            }
            div { class: "card",
                match &*projects.read() {
                    Some(Ok(projects)) => rsx! {
                        div { class: "table-container",
                            table {
                                thead {
                                    tr {
                                        th { "Name" }
                                        th { "Client" }
                                        th { "Billing" }
                                        th { "Status" }
                                        th { "Actions" }
                                    }
                                }
                                tbody {
                                    for project in projects.iter() {
                                        tr { key: "{project.id}",
                                            td { "{project.name}" }
                                            td { "{project.client_id}" }
                                            td { "{project.billing_method}" }
                                            td {
                                                if project.is_active {
                                                    span { class: "badge badge-success", "Active" }
                                                } else {
                                                    span { class: "badge badge-neutral", "Inactive" }
                                                }
                                            }
                                            td {
                                                button { class: "btn btn-secondary btn-sm", "View" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! { div { class: "alert alert-danger", "{e}" } },
                    None => rsx! { div { class: "text-muted text-sm", "Loading..." } },
                }
            }
        }
    }
}

#[component]
pub fn ProjectDetail(id: Uuid) -> Element {
    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Project" }
            }
            div { class: "card",
                p { class: "text-muted", "Project detail for {id}" }
            }
        }
    }
}
