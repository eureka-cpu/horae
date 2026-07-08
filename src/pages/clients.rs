use dioxus::prelude::*;
use uuid::Uuid;

use crate::server_fns;

#[component]
pub fn ClientList() -> Element {
    let clients = use_resource(|| async move { server_fns::list_clients().await });

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Clients" }
                div { class: "page-actions",
                    button { class: "btn btn-primary", "New Client" }
                }
            }
            div { class: "card",
                match &*clients.read() {
                    Some(Ok(clients)) => rsx! {
                        div { class: "table-container",
                            table {
                                thead {
                                    tr {
                                        th { "Name" }
                                        th { "Currency" }
                                        th { "Address" }
                                        th { "Actions" }
                                    }
                                }
                                tbody {
                                    for client in clients.iter() {
                                        tr { key: "{client.id}",
                                            td { "{client.name}" }
                                            td { "{client.currency}" }
                                            td { "{client.address.as_deref().unwrap_or(\"-\")}" }
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
pub fn ClientDetail(id: Uuid) -> Element {
    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Client" }
            }
            div { class: "card",
                p { class: "text-muted", "Client detail for {id}" }
            }
        }
    }
}
