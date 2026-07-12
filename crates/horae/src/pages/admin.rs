use dioxus::prelude::*;

use crate::server_fns;

#[component]
pub fn AdminUsers() -> Element {
    let mut users = use_resource(|| async move { server_fns::list_users(true).await });
    let mut tasks = use_resource(|| async move { server_fns::list_tasks().await });

    let mut show_user_form = use_signal(|| false);
    let mut user_email = use_signal(String::new);
    let mut user_name = use_signal(String::new);
    let mut user_role = use_signal(|| "member".to_string());
    let mut user_error = use_signal(|| None::<String>);

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
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| show_user_form.set(!show_user_form()),
                        if show_user_form() { "Cancel" } else { "Invite User" }
                    }
                }
            }

            if show_user_form() {
                div { class: "card",
                    div { style: "padding: 1.25rem;",
                        h3 { class: "text-sm", style: "margin-bottom: 1rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-text-muted);", "New User" }
                        if let Some(err) = &*user_error.read() {
                            div { class: "alert alert-danger", "{err}" }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "user-email", "Email" }
                            input {
                                class: "form-input",
                                id: "user-email",
                                r#type: "email",
                                placeholder: "user@example.com",
                                value: "{user_email}",
                                oninput: move |e| user_email.set(e.value()),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "user-name", "Name" }
                            input {
                                class: "form-input",
                                id: "user-name",
                                r#type: "text",
                                placeholder: "Full name",
                                value: "{user_name}",
                                oninput: move |e| user_name.set(e.value()),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "user-role", "Role" }
                            select {
                                class: "form-input",
                                id: "user-role",
                                value: "{user_role}",
                                onchange: move |e| user_role.set(e.value()),
                                option { value: "member", "Member" }
                                option { value: "manager", "Manager" }
                                option { value: "admin", "Admin" }
                            }
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| {
                                let e = user_email();
                                let n = user_name();
                                let r = user_role();
                                spawn(async move {
                                    match server_fns::create_user(e, n, r).await {
                                        Ok(_) => {
                                            user_email.set(String::new());
                                            user_name.set(String::new());
                                            user_role.set("member".to_string());
                                            user_error.set(None);
                                            show_user_form.set(false);
                                            users.restart();
                                        }
                                        Err(e) => user_error.set(Some(e.to_string())),
                                    }
                                });
                            },
                            "Create User"
                        }
                    }
                }
            }

            div { class: "card",
                match &*users.read() {
                    Some(Ok(user_list)) => rsx! {
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
                                    for user in user_list.iter() {
                                        {
                                            let uid = user.id.to_string();
                                            let uid2 = uid.clone();
                                            let current_role = user.org_role.to_string();
                                            let is_active = user.active;
                                            rsx! {
                                                tr { key: "{uid}",
                                                    td { "{user.name}" }
                                                    td { "{user.email}" }
                                                    td {
                                                        select {
                                                            class: "form-input",
                                                            style: "width: auto; padding: 0.25rem 0.5rem; font-size: 0.8125rem;",
                                                            value: "{current_role}",
                                                            onchange: {
                                                                let uid = uid.clone();
                                                                move |e: Event<FormData>| {
                                                                    let uid = uid.clone();
                                                                    let new_role = e.value();
                                                                    spawn(async move {
                                                                        let _ = server_fns::set_user_role(uid, new_role).await;
                                                                        users.restart();
                                                                    });
                                                                }
                                                            },
                                                            option { value: "member", "Member" }
                                                            option { value: "manager", "Manager" }
                                                            option { value: "admin", "Admin" }
                                                        }
                                                    }
                                                    td {
                                                        if is_active {
                                                            span { class: "badge badge-success", "Active" }
                                                        } else {
                                                            span { class: "badge badge-neutral", "Inactive" }
                                                        }
                                                    }
                                                    td {
                                                        button {
                                                            class: if is_active { "btn btn-secondary btn-sm" } else { "btn btn-primary btn-sm" },
                                                            onclick: {
                                                                let uid = uid2.clone();
                                                                move |_| {
                                                                    let uid = uid.clone();
                                                                    let new_active = !is_active;
                                                                    spawn(async move {
                                                                        let _ = server_fns::set_user_active(uid, new_active).await;
                                                                        users.restart();
                                                                    });
                                                                }
                                                            },
                                                            if is_active { "Deactivate" } else { "Activate" }
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
