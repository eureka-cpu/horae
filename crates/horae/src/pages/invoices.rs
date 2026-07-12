use std::collections::HashMap;

use dioxus::prelude::*;
use horae_core::types::InvoiceStatus;
use uuid::Uuid;

use crate::route::Route;
use crate::server_fns;

#[component]
pub fn InvoiceList() -> Element {
    let mut invoices = use_resource(|| async move { server_fns::list_invoices(None).await });
    let clients = use_resource(|| async move { server_fns::list_clients(false).await });

    let client_names: HashMap<Uuid, String> = clients
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|cs| cs.iter().map(|c| (c.id, c.name.clone())).collect())
        .unwrap_or_default();

    let client_list: Vec<(Uuid, String)> = clients
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|cs| cs.iter().map(|c| (c.id, c.name.clone())).collect())
        .unwrap_or_default();

    let mut show_form = use_signal(|| false);
    let mut selected_client = use_signal(String::new);
    let mut period_from = use_signal(String::new);
    let mut period_to = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Invoices" }
                div { class: "page-actions",
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            if show_form() {
                                show_form.set(false);
                                error.set(None);
                            } else {
                                show_form.set(true);
                            }
                        },
                        if show_form() { "Cancel" } else { "New Invoice" }
                    }
                }
            }

            if show_form() {
                div { class: "card",
                    div { style: "padding: 1.25rem;",
                        h3 { class: "text-sm", style: "margin-bottom: 1rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-text-muted);",
                            "Generate Invoice"
                        }
                        if let Some(err) = &*error.read() {
                            div { class: "alert alert-danger", "{err}" }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "inv-client", "Client" }
                            select {
                                class: "form-input",
                                id: "inv-client",
                                value: "{selected_client}",
                                onchange: move |e| selected_client.set(e.value()),
                                option { value: "", "Select a client…" }
                                for (cid, cname) in client_list.iter() {
                                    option { value: "{cid}", "{cname}" }
                                }
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "inv-from", "Period from" }
                            input {
                                class: "form-input",
                                id: "inv-from",
                                r#type: "date",
                                value: "{period_from}",
                                oninput: move |e| period_from.set(e.value()),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "inv-to", "Period to" }
                            input {
                                class: "form-input",
                                id: "inv-to",
                                r#type: "date",
                                value: "{period_to}",
                                oninput: move |e| period_to.set(e.value()),
                            }
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| {
                                let client = selected_client();
                                let from = period_from();
                                let to = period_to();
                                spawn(async move {
                                    match server_fns::generate_invoice(client, from, to).await {
                                        Ok(_) => {
                                            show_form.set(false);
                                            error.set(None);
                                            invoices.restart();
                                        }
                                        Err(e) => error.set(Some(e.to_string())),
                                    }
                                });
                            },
                            "Generate Invoice"
                        }
                    }
                }
            }

            div { class: "card",
                match &*invoices.read() {
                    Some(Ok(invoices)) => rsx! {
                        if invoices.is_empty() {
                            div { class: "text-muted text-sm", style: "padding: 1.25rem;",
                                "No invoices yet. Generate one from billable time."
                            }
                        } else {
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
                                            {
                                                let inv = invoice.clone();
                                                rsx! {
                                                    tr { key: "{inv.id}",
                                                        td { class: "text-mono", "{inv.number}" }
                                                        td {
                                                            {client_names.get(&inv.client_id)
                                                                .cloned()
                                                                .unwrap_or_else(|| inv.client_id.to_string())}
                                                        }
                                                        td {
                                                            span {
                                                                class: match inv.status {
                                                                    InvoiceStatus::Paid => "badge badge-success",
                                                                    InvoiceStatus::Sent => "badge badge-info",
                                                                    InvoiceStatus::Void => "badge badge-neutral",
                                                                    InvoiceStatus::Draft => "badge badge-warning",
                                                                },
                                                                "{inv.status}"
                                                            }
                                                        }
                                                        td { class: "text-mono", "{inv.issued_on}" }
                                                        td { class: "text-mono", "{inv.due_on}" }
                                                        td { class: "text-mono text-right",
                                                            { format!("{} {:.2}", inv.currency.trim(), inv.total_cents as f64 / 100.0) }
                                                        }
                                                        td {
                                                            Link {
                                                                to: Route::InvoiceDetail { id: inv.id },
                                                                class: "btn btn-secondary btn-sm",
                                                                "View"
                                                            }
                                                        }
                                                    }
                                                }
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
    let mut invoice_data =
        use_resource(move || async move { server_fns::get_invoice(id.to_string()).await });
    let clients = use_resource(|| async move { server_fns::list_clients(false).await });
    let mut error = use_signal(|| None::<String>);

    let client_name = |cid: Uuid| -> String {
        clients
            .read()
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .and_then(|cs| cs.iter().find(|c| c.id == cid))
            .map(|c| c.name.clone())
            .unwrap_or_else(|| cid.to_string())
    };

    rsx! {
        div {
            match &*invoice_data.read() {
                Some(Ok(data)) => {
                    let inv = data.invoice.clone();
                    let lines = data.lines.clone();
                    let cname = client_name(inv.client_id);

                    rsx! {
                        div { class: "page-header",
                            h1 { class: "page-title", "Invoice {inv.number}" }
                            div { class: "page-actions",
                                match inv.status {
                                    InvoiceStatus::Draft => rsx! {
                                        button {
                                            class: "btn btn-primary",
                                            onclick: {
                                                let iid = inv.id.to_string();
                                                move |_| {
                                                    let iid = iid.clone();
                                                    spawn(async move {
                                                        match server_fns::update_invoice_status(iid, "sent".to_string()).await {
                                                            Ok(_) => { invoice_data.restart(); error.set(None); }
                                                            Err(e) => error.set(Some(e.to_string())),
                                                        }
                                                    });
                                                }
                                            },
                                            "Mark Sent"
                                        }
                                        button {
                                            class: "btn btn-secondary",
                                            style: "margin-left: 0.5rem;",
                                            onclick: {
                                                let iid = inv.id.to_string();
                                                move |_| {
                                                    let iid = iid.clone();
                                                    spawn(async move {
                                                        match server_fns::update_invoice_status(iid, "void".to_string()).await {
                                                            Ok(_) => { invoice_data.restart(); error.set(None); }
                                                            Err(e) => error.set(Some(e.to_string())),
                                                        }
                                                    });
                                                }
                                            },
                                            "Void"
                                        }
                                    },
                                    InvoiceStatus::Sent => rsx! {
                                        button {
                                            class: "btn btn-primary",
                                            onclick: {
                                                let iid = inv.id.to_string();
                                                move |_| {
                                                    let iid = iid.clone();
                                                    spawn(async move {
                                                        match server_fns::update_invoice_status(iid, "paid".to_string()).await {
                                                            Ok(_) => { invoice_data.restart(); error.set(None); }
                                                            Err(e) => error.set(Some(e.to_string())),
                                                        }
                                                    });
                                                }
                                            },
                                            "Mark Paid"
                                        }
                                        button {
                                            class: "btn btn-secondary",
                                            style: "margin-left: 0.5rem;",
                                            onclick: {
                                                let iid = inv.id.to_string();
                                                move |_| {
                                                    let iid = iid.clone();
                                                    spawn(async move {
                                                        match server_fns::update_invoice_status(iid, "void".to_string()).await {
                                                            Ok(_) => { invoice_data.restart(); error.set(None); }
                                                            Err(e) => error.set(Some(e.to_string())),
                                                        }
                                                    });
                                                }
                                            },
                                            "Void"
                                        }
                                    },
                                    _ => rsx! {},
                                }
                                a {
                                    class: "btn btn-secondary",
                                    style: "margin-left: 0.5rem;",
                                    href: "/api/invoices/{inv.id}/export/pdf",
                                    target: "_blank",
                                    "Download PDF"
                                }
                            }
                        }

                        if let Some(err) = &*error.read() {
                            div { class: "alert alert-danger", "{err}" }
                        }

                        div { class: "card", style: "margin-bottom: 1.5rem;",
                            div { style: "padding: 1.25rem;",
                                div { style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 1rem;",
                                    div {
                                        div { class: "text-sm text-muted", "Client" }
                                        div { "{cname}" }
                                    }
                                    div {
                                        div { class: "text-sm text-muted", "Status" }
                                        span {
                                            class: match inv.status {
                                                InvoiceStatus::Paid => "badge badge-success",
                                                InvoiceStatus::Sent => "badge badge-info",
                                                InvoiceStatus::Void => "badge badge-neutral",
                                                InvoiceStatus::Draft => "badge badge-warning",
                                            },
                                            "{inv.status}"
                                        }
                                    }
                                    div {
                                        div { class: "text-sm text-muted", "Issued" }
                                        div { class: "text-mono", "{inv.issued_on}" }
                                    }
                                    div {
                                        div { class: "text-sm text-muted", "Due" }
                                        div { class: "text-mono", "{inv.due_on}" }
                                    }
                                    div {
                                        div { class: "text-sm text-muted", "Total" }
                                        div { class: "text-mono",
                                            { format!("{} {:.2}", inv.currency.trim(), inv.total_cents as f64 / 100.0) }
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "card",
                            div { class: "table-container",
                                table {
                                    thead {
                                        tr {
                                            th { "Description" }
                                            th { class: "text-right", "Hours" }
                                            th { class: "text-right", "Rate" }
                                            th { class: "text-right", "Amount" }
                                        }
                                    }
                                    tbody {
                                        for line in lines.iter() {
                                            tr { key: "{line.id}",
                                                td { "{line.description}" }
                                                td { class: "text-mono text-right",
                                                    { horae_core::duration::format_hhmm(line.minutes as u32) }
                                                }
                                                td { class: "text-mono text-right",
                                                    { format!("{:.2}/hr", line.rate_cents as f64 / 100.0) }
                                                }
                                                td { class: "text-mono text-right",
                                                    { format!("{:.2}", line.amount_cents as f64 / 100.0) }
                                                }
                                            }
                                        }
                                    }
                                    tfoot {
                                        tr {
                                            td { colspan: "3", class: "text-right", style: "font-weight: 600;", "Total" }
                                            td { class: "text-mono text-right", style: "font-weight: 600;",
                                                { format!("{} {:.2}", inv.currency.trim(), inv.total_cents as f64 / 100.0) }
                                            }
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
