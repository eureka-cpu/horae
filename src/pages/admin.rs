use dioxus::prelude::*;

use crate::server_fns;

#[component]
pub fn AdminUsers() -> Element {
    let users = use_resource(|| async move { server_fns::list_users().await });
    let mut tasks = use_resource(|| async move { server_fns::list_tasks(None).await });

    let mut show_task_form = use_signal(|| false);
    let mut task_name = use_signal(String::new);
    let mut task_billable = use_signal(|| true);
    let mut task_error = use_signal(|| None::<String>);

    rsx! {
        div {
            // ── Users section ───────────────────────────────────────────
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

            // ── Tasks section ───────────────────────────────────────────
            div { style: "margin-top: 2rem;",
                div { class: "page-header",
                    h2 { class: "page-title", style: "font-size: 1.25rem;", "Tasks" }
                    div { class: "page-actions",
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| show_task_form.set(!show_task_form()),
                            if show_task_form() { "Cancel" } else { "Add Task" }
                        }
                    }
                }

                if show_task_form() {
                    div { class: "card",
                        div { style: "padding: 1.25rem;",
                            h3 { class: "text-sm", style: "margin-bottom: 1rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-text-muted);", "New Task" }
                            if let Some(err) = &*task_error.read() {
                                div { class: "alert alert-danger", "{err}" }
                            }
                            div { class: "form-group",
                                label { class: "form-label", r#for: "task-name", "Name" }
                                input {
                                    class: "form-input",
                                    id: "task-name",
                                    r#type: "text",
                                    placeholder: "Task name",
                                    value: "{task_name}",
                                    oninput: move |e| task_name.set(e.value()),
                                }
                            }
                            div { class: "form-group",
                                label { class: "form-label", style: "display: flex; align-items: center; gap: 0.5rem;",
                                    input {
                                        r#type: "checkbox",
                                        checked: task_billable(),
                                        onchange: move |e| task_billable.set(e.checked()),
                                    }
                                    "Billable by default"
                                }
                            }
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| {
                                    let n = task_name();
                                    let b = task_billable();
                                    spawn(async move {
                                        match server_fns::create_task(n, b).await {
                                            Ok(_) => {
                                                task_name.set(String::new());
                                                task_billable.set(true);
                                                task_error.set(None);
                                                show_task_form.set(false);
                                                tasks.restart();
                                            }
                                            Err(e) => task_error.set(Some(e.to_string())),
                                        }
                                    });
                                },
                                "Create Task"
                            }
                        }
                    }
                }

                div { class: "card",
                    match &*tasks.read() {
                        Some(Ok(task_list)) if task_list.is_empty() => rsx! {
                            p { class: "text-muted text-sm", style: "padding: 1.25rem;", "No tasks defined yet." }
                        },
                        Some(Ok(task_list)) => rsx! {
                            div { class: "table-container",
                                table {
                                    thead {
                                        tr {
                                            th { "Name" }
                                            th { "Billable Default" }
                                            th { "Active" }
                                        }
                                    }
                                    tbody {
                                        for task in task_list.iter() {
                                            tr { key: "{task.id}",
                                                td { "{task.name}" }
                                                td {
                                                    if task.billable_default {
                                                        span { class: "badge badge-success", "Yes" }
                                                    } else {
                                                        span { class: "badge badge-neutral", "No" }
                                                    }
                                                }
                                                td {
                                                    if task.active {
                                                        span { class: "badge badge-success", "Active" }
                                                    } else {
                                                        span { class: "badge badge-neutral", "Inactive" }
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
}
