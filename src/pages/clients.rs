use dioxus::prelude::*;
use uuid::Uuid;

use crate::server_fns;

#[component]
pub fn ClientList() -> Element {
    let mut clients = use_resource(|| async move { server_fns::list_clients().await });
    let me = use_resource(|| async move { server_fns::get_me().await });

    let mut show_form = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut currency = use_signal(|| "USD".to_string());
    let mut address = use_signal(String::new);
    let mut tax_id = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);

    let is_admin = match &*me.read() {
        Some(Ok(user)) => user.is_admin(),
        _ => false,
    };

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Clients" }
                div { class: "page-actions",
                    if is_admin {
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| show_form.set(!show_form()),
                            if show_form() { "Cancel" } else { "Add Client" }
                        }
                    }
                }
            }

            if show_form() && is_admin {
                div { class: "card",
                    div { style: "padding: 1.25rem;",
                        h3 { class: "text-sm", style: "margin-bottom: 1rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-text-muted);", "New Client" }
                        if let Some(err) = &*error.read() {
                            div { class: "alert alert-danger", "{err}" }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "client-name", "Name" }
                            input {
                                class: "form-input",
                                id: "client-name",
                                r#type: "text",
                                placeholder: "Client name",
                                value: "{name}",
                                oninput: move |e| name.set(e.value()),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "client-currency", "Currency" }
                            input {
                                class: "form-input",
                                id: "client-currency",
                                r#type: "text",
                                placeholder: "USD",
                                value: "{currency}",
                                oninput: move |e| currency.set(e.value()),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "client-address", "Address (optional)" }
                            textarea {
                                class: "form-input",
                                id: "client-address",
                                placeholder: "Client address",
                                value: "{address}",
                                oninput: move |e| address.set(e.value()),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "client-taxid", "Tax ID (optional)" }
                            input {
                                class: "form-input",
                                id: "client-taxid",
                                r#type: "text",
                                placeholder: "Tax ID",
                                value: "{tax_id}",
                                oninput: move |e| tax_id.set(e.value()),
                            }
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| {
                                let n = name();
                                let c = currency();
                                let a = address();
                                let t = tax_id();
                                spawn(async move {
                                    let addr = if a.is_empty() { None } else { Some(a) };
                                    let tid = if t.is_empty() { None } else { Some(t) };
                                    match server_fns::create_client(n, c, addr, tid).await {
                                        Ok(_) => {
                                            name.set(String::new());
                                            currency.set("USD".to_string());
                                            address.set(String::new());
                                            tax_id.set(String::new());
                                            error.set(None);
                                            show_form.set(false);
                                            clients.restart();
                                        }
                                        Err(e) => error.set(Some(e.to_string())),
                                    }
                                });
                            },
                            "Create Client"
                        }
                    }
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
