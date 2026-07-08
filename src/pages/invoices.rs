use std::collections::HashMap;

use dioxus::prelude::*;
use uuid::Uuid;

use crate::server_fns;

#[component]
pub fn InvoiceList() -> Element {
    let invoices = use_resource(|| async move { server_fns::list_invoices(None).await });
    let clients = use_resource(|| async move { server_fns::list_clients().await });

    let client_names: HashMap<Uuid, String> = clients
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|cs| cs.iter().map(|c| (c.id, c.name.clone())).collect())
        .unwrap_or_default();

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Invoices" }
                div { class: "page-actions",
                    button { class: "btn btn-primary", "New Invoice" }
                }
            }
            div { class: "card",
                match &*invoices.read() {
                    Some(Ok(invoices)) => rsx! {
                        div { class: "table-container",
                            table {
                                thead {
                                    tr {
                                        th { "Number" }
                                        th { "Client" }
                                        th { "Status" }
                                        th { "Issued" }
                                        th { "Due" }
                                        th { "Total" }
                                        th { "Actions" }
                                    }
                                }
                                tbody {
                                    for invoice in invoices.iter() {
                                        tr { key: "{invoice.id}",
                                            td { "{invoice.invoice_number}" }
                                            td {
                                                {client_names.get(&invoice.client_id)
                                                    .cloned()
                                                    .unwrap_or_else(|| invoice.client_id.to_string())}
                                            }
                                            td {
                                                span {
                                                    class: match invoice.status.as_str() {
                                                        "paid" => "badge badge-success",
                                                        "sent" => "badge badge-info",
                                                        "void" => "badge badge-neutral",
                                                        _ => "badge badge-warning",
                                                    },
                                                    "{invoice.status}"
                                                }
                                            }
                                            td { "{invoice.issued_date}" }
                                            td { "{invoice.due_date}" }
                                            td { "${invoice.total_amount:.2}" }
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
pub fn InvoiceDetail(id: Uuid) -> Element {
    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Invoice" }
            }
            div { class: "card",
                p { class: "text-muted", "Invoice detail for {id}" }
            }
        }
    }
}
