use dioxus::prelude::*;
use uuid::Uuid;

use crate::route::Route;
use crate::server_fns;

#[component]
pub fn ProjectList() -> Element {
    let mut projects = use_resource(|| async move { server_fns::list_projects(None, None).await });
    let clients_res = use_resource(|| async move { server_fns::list_clients().await });
    let me = use_resource(|| async move { server_fns::get_me().await });

    let mut show_form = use_signal(|| false);
    let mut client_id = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut project_type = use_signal(|| "time_and_materials".to_string());
    let mut currency = use_signal(|| "USD".to_string());
    let mut budget_kind = use_signal(|| "none".to_string());
    let mut error = use_signal(|| None::<String>);

    let is_admin = match &*me.read() {
        Some(Ok(user)) => user.is_admin(),
        _ => false,
    };

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Projects" }
                div { class: "page-actions",
                    if is_admin {
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| show_form.set(!show_form()),
                            if show_form() { "Cancel" } else { "Add Project" }
                        }
                    }
                }
            }

            if show_form() && is_admin {
                div { class: "card",
                    div { style: "padding: 1.25rem;",
                        h3 { class: "text-sm", style: "margin-bottom: 1rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-text-muted);", "New Project" }
                        if let Some(err) = &*error.read() {
                            div { class: "alert alert-danger", "{err}" }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "proj-client", "Client" }
                            select {
                                class: "form-input",
                                id: "proj-client",
                                value: "{client_id}",
                                oninput: move |e| client_id.set(e.value()),
                                option { value: "", "Select a client..." }
                                if let Some(Ok(clients)) = &*clients_res.read() {
                                    for c in clients.iter() {
                                        option { value: "{c.id}", "{c.name}" }
                                    }
                                }
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "proj-name", "Name" }
                            input {
                                class: "form-input",
                                id: "proj-name",
                                r#type: "text",
                                placeholder: "Project name",
                                value: "{name}",
                                oninput: move |e| name.set(e.value()),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "proj-type", "Type" }
                            select {
                                class: "form-input",
                                id: "proj-type",
                                value: "{project_type}",
                                oninput: move |e| project_type.set(e.value()),
                                option { value: "time_and_materials", "Time & Materials" }
                                option { value: "fixed_fee", "Fixed Fee" }
                                option { value: "non_billable", "Non-Billable" }
                                option { value: "retainer", "Retainer" }
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "proj-currency", "Currency" }
                            input {
                                class: "form-input",
                                id: "proj-currency",
                                r#type: "text",
                                placeholder: "USD",
                                value: "{currency}",
                                oninput: move |e| currency.set(e.value()),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label", r#for: "proj-budget", "Budget" }
                            select {
                                class: "form-input",
                                id: "proj-budget",
                                value: "{budget_kind}",
                                oninput: move |e| budget_kind.set(e.value()),
                                option { value: "none", "None" }
                                option { value: "amount", "Amount" }
                                option { value: "hours", "Hours" }
                            }
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| {
                                let cid = client_id();
                                let n = name();
                                let pt = project_type();
                                let c = currency();
                                let bk = budget_kind();
                                spawn(async move {
                                    match server_fns::create_project(cid, n, pt, c, bk).await {
                                        Ok(_) => {
                                            client_id.set(String::new());
                                            name.set(String::new());
                                            project_type.set("time_and_materials".to_string());
                                            currency.set("USD".to_string());
                                            budget_kind.set("none".to_string());
                                            error.set(None);
                                            show_form.set(false);
                                            projects.restart();
                                        }
                                        Err(e) => error.set(Some(e.to_string())),
                                    }
                                });
                            },
                            "Create Project"
                        }
                    }
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
                                            td { "{project.project_type}" }
                                            td {
                                                if project.active {
                                                    span { class: "badge badge-success", "Active" }
                                                } else {
                                                    span { class: "badge badge-neutral", "Inactive" }
                                                }
                                            }
                                            td {
                                                Link {
                                                    to: Route::ProjectDetail { id: project.id },
                                                    class: "btn btn-secondary btn-sm",
                                                    "View"
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
pub fn ProjectDetail(id: Uuid) -> Element {
    let me = use_resource(|| async move { server_fns::get_me().await });
    let mut assignments = use_resource(move || {
        let pid = id.to_string();
        async move { server_fns::list_assignments(pid).await }
    });
    let users_res = use_resource(|| async move { server_fns::list_users().await });

    let mut show_assign_form = use_signal(|| false);
    let mut assign_user_id = use_signal(String::new);
    let mut assign_role = use_signal(|| "freelancer".to_string());
    let mut error = use_signal(|| None::<String>);

    let is_admin = match &*me.read() {
        Some(Ok(user)) => user.is_admin(),
        _ => false,
    };

    // Build a lookup from user_id -> user name
    let users_map: std::collections::HashMap<uuid::Uuid, String> = match &*users_res.read() {
        Some(Ok(users)) => users.iter().map(|u| (u.id, u.name.clone())).collect(),
        _ => std::collections::HashMap::new(),
    };

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Project" }
            }
            div { class: "card",
                p { class: "text-muted", style: "padding: 1.25rem;", "Project detail for {id}" }
            }

            // ── Assignments section ─────────────────────────────────────
            div { style: "margin-top: 1.5rem;",
                div { class: "page-header",
                    h2 { class: "page-title", style: "font-size: 1.25rem;", "Assignments" }
                    div { class: "page-actions",
                        if is_admin {
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| show_assign_form.set(!show_assign_form()),
                                if show_assign_form() { "Cancel" } else { "Assign User" }
                            }
                        }
                    }
                }

                if show_assign_form() && is_admin {
                    div { class: "card",
                        div { style: "padding: 1.25rem;",
                            h3 { class: "text-sm", style: "margin-bottom: 1rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-text-muted);", "Assign User" }
                            if let Some(err) = &*error.read() {
                                div { class: "alert alert-danger", "{err}" }
                            }
                            div { class: "form-group",
                                label { class: "form-label", r#for: "assign-user", "User" }
                                select {
                                    class: "form-input",
                                    id: "assign-user",
                                    value: "{assign_user_id}",
                                    oninput: move |e| assign_user_id.set(e.value()),
                                    option { value: "", "Select a user..." }
                                    if let Some(Ok(users)) = &*users_res.read() {
                                        for u in users.iter() {
                                            option { value: "{u.id}", "{u.name} ({u.email})" }
                                        }
                                    }
                                }
                            }
                            div { class: "form-group",
                                label { class: "form-label", r#for: "assign-role", "Role" }
                                select {
                                    class: "form-input",
                                    id: "assign-role",
                                    value: "{assign_role}",
                                    oninput: move |e| assign_role.set(e.value()),
                                    option { value: "lead", "Lead" }
                                    option { value: "freelancer", "Freelancer" }
                                    option { value: "admin", "Admin" }
                                }
                            }
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| {
                                    let pid = id.to_string();
                                    let uid = assign_user_id();
                                    let r = assign_role();
                                    spawn(async move {
                                        match server_fns::create_assignment(pid, uid, r).await {
                                            Ok(_) => {
                                                assign_user_id.set(String::new());
                                                assign_role.set("freelancer".to_string());
                                                error.set(None);
                                                show_assign_form.set(false);
                                                assignments.restart();
                                            }
                                            Err(e) => error.set(Some(e.to_string())),
                                        }
                                    });
                                },
                                "Assign"
                            }
                        }
                    }
                }

                div { class: "card",
                    match &*assignments.read() {
                        Some(Ok(list)) if list.is_empty() => rsx! {
                            p { class: "text-muted text-sm", style: "padding: 1.25rem;", "No users assigned yet." }
                        },
                        Some(Ok(list)) => rsx! {
                            div { class: "table-container",
                                table {
                                    thead {
                                        tr {
                                            th { "User" }
                                            th { "Role" }
                                            if is_admin {
                                                th { "Actions" }
                                            }
                                        }
                                    }
                                    tbody {
                                        for a in list.iter() {
                                            {
                                                let aid = a.id.to_string();
                                                let user_name = users_map.get(&a.user_id).cloned().unwrap_or_else(|| a.user_id.to_string());
                                                rsx! {
                                                    tr { key: "{a.id}",
                                                        td { "{user_name}" }
                                                        td { "{a.role}" }
                                                        if is_admin {
                                                            td {
                                                                button {
                                                                    class: "btn btn-danger btn-sm",
                                                                    onclick: move |_| {
                                                                        let aid = aid.clone();
                                                                        spawn(async move {
                                                                            if let Err(e) = server_fns::delete_assignment(aid).await {
                                                                                error.set(Some(e.to_string()));
                                                                            } else {
                                                                                assignments.restart();
                                                                            }
                                                                        });
                                                                    },
                                                                    "Remove"
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
}
