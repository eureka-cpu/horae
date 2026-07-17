use dioxus::prelude::*;
use uuid::Uuid;

use crate::server_fns;

#[component]
pub fn ClientList() -> Element {
    // Management view: include inactive clients so managers can reactivate them.
    let mut clients = use_resource(|| async move { server_fns::list_clients(true).await });
    let me = use_resource(|| async move { server_fns::get_me().await });

    let mut show_form = use_signal(|| false);
    // `Some(id)` while editing an existing client, `None` while creating.
    let mut editing_id = use_signal(|| None::<Uuid>);
    let mut name = use_signal(String::new);
    let mut currency = use_signal(|| "USD".to_string());
    let mut address = use_signal(String::new);
    let mut tax_id = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);

    let is_manager = match &*me.read() {
        Some(Ok(user)) => user.is_manager_or_above(),
        _ => false,
    };

    let mut reset_form = move || {
        editing_id.set(None);
        name.set(String::new());
        currency.set("USD".to_string());
        address.set(String::new());
        tax_id.set(String::new());
        error.set(None);
        show_form.set(false);
    };

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Clients" }
                div { class: "page-actions",
                    if is_manager {
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| {
                                if show_form() {
                                    reset_form();
                                } else {
                                    editing_id.set(None);
                                    show_form.set(true);
                                }
                            },
                            if show_form() { "Cancel" } else { "Add Client" }
                        }
                    }
                }
            }

            if show_form() && is_manager {
                div { class: "card",
                    div { class: "p-5",
                        h3 { class: "text-sm mb-4 uppercase tracking-wide text-faint",
                            if editing_id().is_some() { "Edit Client" } else { "New Client" }
                        }
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
                                let editing = editing_id();
                                let n = name();
                                let c = currency();
                                let a = address();
                                let t = tax_id();
                                spawn(async move {
                                    let addr = if a.is_empty() { None } else { Some(a) };
                                    let tid = if t.is_empty() { None } else { Some(t) };
                                    let result = match editing {
                                        Some(id) => {
                                            server_fns::update_client(id.to_string(), n, c, addr, tid).await
                                        }
                                        None => server_fns::create_client(n, c, addr, tid).await,
                                    };
                                    match result {
                                        Ok(_) => {
                                            reset_form();
                                            clients.restart();
                                        }
                                        Err(e) => error.set(Some(e.to_string())),
                                    }
                                });
                            },
                            if editing_id().is_some() { "Save Changes" } else { "Create Client" }
                        }
                    }
                }
            }

            div { class: "card",
                match &*clients.read() {
                    Some(Ok(list)) => rsx! {
                        div { class: "table-container",
                            table {
                                thead {
                                    tr {
                                        th { "Name" }
                                        th { "Currency" }
                                        th { "Address" }
                                        th { "Status" }
                                        th { "Actions" }
                                    }
                                }
                                tbody {
                                    for client in list.iter() {
                                        {
                                            let c = client.clone();
                                            rsx! {
                                                tr { key: "{c.id}",
                                                    td { "{c.name}" }
                                                    td { "{c.currency}" }
                                                    td { "{c.address.as_deref().unwrap_or(\"-\")}" }
                                                    td {
                                                        if c.active {
                                                            span { class: "badge badge-success", "Active" }
                                                        } else {
                                                            span { class: "badge", "Inactive" }
                                                        }
                                                    }
                                                    td {
                                                        if is_manager {
                                                            button {
                                                                class: "btn btn-secondary btn-sm",
                                                                onclick: {
                                                                    let c = c.clone();
                                                                    move |_| {
                                                                        editing_id.set(Some(c.id));
                                                                        name.set(c.name.clone());
                                                                        currency.set(c.currency.clone());
                                                                        address.set(c.address.clone().unwrap_or_default());
                                                                        tax_id.set(c.tax_id.clone().unwrap_or_default());
                                                                        error.set(None);
                                                                        show_form.set(true);
                                                                    }
                                                                },
                                                                "Edit"
                                                            }
                                                            button {
                                                                class: "btn btn-secondary btn-sm",
                                                                style: "margin-left: 0.5rem;",
                                                                onclick: {
                                                                    let id = c.id;
                                                                    let next_active = !c.active;
                                                                    move |_| {
                                                                        spawn(async move {
                                                                            match server_fns::set_client_active(id.to_string(), next_active).await {
                                                                                Ok(_) => clients.restart(),
                                                                                Err(e) => error.set(Some(e.to_string())),
                                                                            }
                                                                        });
                                                                    }
                                                                },
                                                                if c.active { "Deactivate" } else { "Activate" }
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
