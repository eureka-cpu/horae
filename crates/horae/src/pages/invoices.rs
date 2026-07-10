use std::collections::HashMap;

use dioxus::prelude::*;
use uuid::Uuid;

use crate::server_fns;

#[component]
pub fn InvoiceList() -> Element {
    let invoices = use_resource(|| async move { server_fns::list_invoices(None).await });
    let clients = use_resource(|| async move { server_fns::list_clients(None).await });

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
                                        th { class: "text-right", "Total" }
                                        th { "Actions" }
                                    }
                                }
                                tbody {
                                    for invoice in invoices.iter() {
                                        tr { key: "{invoice.id}",
                                            td { class: "text-mono", "{invoice.invoice_number}" }
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
                                            td { class: "text-mono", "{invoice.issued_date}" }
                                            td { class: "text-mono", "{invoice.due_date}" }
                                            td { class: "text-mono text-right",
                                                { format!("${:.2}", invoice.total_amount_cents as f64 / 100.0) }
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
