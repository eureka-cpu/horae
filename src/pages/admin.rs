use dioxus::prelude::*;

use crate::server_fns;

#[component]
pub fn AdminUsers() -> Element {
    let users = use_resource(|| async move { server_fns::list_users().await });

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "User Management" }
                div { class: "page-actions",
                    button { class: "btn btn-primary", "Invite User" }
                }
            }
            div { class: "card",
                match &*users.read() {
                    Some(Ok(users)) => rsx! {
                        div { class: "table-container",
                            table {
                                thead {
                                    tr {
                                        th { "Name" }
                                        th { "Email" }
                                        th { "Role" }
                                        th { "Status" }
                                        th { "Actions" }
                                    }
                                }
                                tbody {
                                    for user in users.iter() {
                                        tr { key: "{user.id}",
                                            td { "{user.name}" }
                                            td { "{user.email}" }
                                            td { "{user.org_role}" }
                                            td {
                                                if user.active {
                                                    span { class: "badge badge-success", "Active" }
                                                } else {
                                                    span { class: "badge badge-neutral", "Inactive" }
                                                }
                                            }
                                            td {
                                                button { class: "btn btn-secondary btn-sm", "Edit" }
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
