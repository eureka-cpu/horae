use dioxus::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::Project;
use crate::route::Route;
use crate::server_fns;
use horae_core::types::{BudgetKind, ProjectType};

/// Human label for the project-type pill (the `Display` impl is the snake_case
/// wire form, which isn't what we want to show).
fn type_label(t: ProjectType) -> &'static str {
    match t {
        ProjectType::TimeAndMaterials => "Time & Materials",
        ProjectType::FixedFee => "Fixed Fee",
        ProjectType::NonBillable => "Non-Billable",
        ProjectType::Retainer => "Retainer",
    }
}

/// Budget in the project's own unit. Monetary *spend* isn't resolved yet
/// (needs FR-024 rate resolution), so this shows the configured budget only.
fn budget_display(p: &Project) -> String {
    match p.budget_kind {
        BudgetKind::Amount => p
            .budget_amount_cents
            .map(|c| format!("{} {:.2}", p.currency.trim(), c as f64 / 100.0))
            .unwrap_or_else(|| "—".to_string()),
        BudgetKind::Hours => p
            .budget_minutes
            .map(|m| format!("{}h", horae_core::duration::format_decimal(m.max(0) as u32)))
            .unwrap_or_else(|| "—".to_string()),
        BudgetKind::None => "—".to_string(),
    }
}

#[component]
pub fn ProjectList() -> Element {
    // Management view: `include_inactive = true` also lists deactivated projects
    // so managers can reactivate them; new-entry pickers pass `false`.
    let mut projects = use_resource(|| async move { server_fns::list_projects(None, true).await });
    // All clients (including inactive) so a project under a deactivated client
    // still resolves to its real name; the create form filters to active ones.
    let clients_res = use_resource(|| async move { server_fns::list_clients(true).await });
    let me = use_resource(|| async move { server_fns::get_me().await });

    let mut show_form = use_signal(|| false);
    // `Some(id)` while editing an existing project, `None` while creating.
    let mut editing_id = use_signal(|| None::<Uuid>);
    let mut client_id = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut project_type = use_signal(|| "time_and_materials".to_string());
    let mut currency = use_signal(|| "USD".to_string());
    let mut budget_kind = use_signal(|| "none".to_string());
    let mut error = use_signal(|| None::<String>);

    // Filters over the loaded list (client-side; the design's status/client
    // dropdowns and search all narrow the same set).
    let mut query = use_signal(String::new);
    // Default to all projects so deactivated ones stay visible for reactivation.
    let mut status_all = use_signal(|| true);
    let mut client_filter = use_signal(String::new);

    let is_manager = match &*me.read() {
        Some(Ok(user)) => user.is_manager_or_above(),
        _ => false,
    };

    let client_names: HashMap<Uuid, String> = match &*clients_res.read() {
        Some(Ok(cs)) => cs.iter().map(|c| (c.id, c.name.clone())).collect(),
        _ => HashMap::new(),
    };
    let (active_count, total_count) = match &*projects.read() {
        Some(Ok(list)) => (list.iter().filter(|p| p.active).count(), list.len()),
        _ => (0, 0),
    };
    let status_val = if status_all() { "all" } else { "active" };

    let mut reset_form = move || {
        editing_id.set(None);
        client_id.set(String::new());
        name.set(String::new());
        project_type.set("time_and_materials".to_string());
        currency.set("USD".to_string());
        budget_kind.set("none".to_string());
        error.set(None);
        show_form.set(false);
    };

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Projects" }
                div { class: "proj-search ml-auto",
                    span { class: "proj-search-icon", aria_hidden: "true", "⌕" }
                    input {
                        class: "proj-search-input",
                        r#type: "text",
                        placeholder: "Search by project or client",
                        aria_label: "Search by project or client",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                    }
                }
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
                        if show_form() { "Cancel" } else { "Add Project" }
                    }
                }
            }

            div { class: "flex items-center gap-4 mb-6",
                select {
                    class: "form-input",
                    aria_label: "Filter by status",
                    value: "{status_val}",
                    oninput: move |e| status_all.set(e.value() == "all"),
                    option { value: "active", "Active projects ({active_count})" }
                    option { value: "all", "All projects ({total_count})" }
                }
                div { class: "flex-1" }
                select {
                    class: "form-input",
                    aria_label: "Filter by client",
                    value: "{client_filter}",
                    oninput: move |e| client_filter.set(e.value()),
                    option { value: "", "All clients" }
                    if let Some(Ok(clients)) = &*clients_res.read() {
                        for c in clients.iter() {
                            option { value: "{c.id}", "{c.name}" }
                        }
                    }
                }
            }

            if show_form() && is_manager {
                div { class: "card",
                    div { class: "p-5",
                        h3 { class: "text-sm mb-4 uppercase tracking-wide text-faint",
                            if editing_id().is_some() { "Edit Project" } else { "New Project" }
                        }
                        if let Some(err) = &*error.read() {
                            div { class: "alert alert-danger", "{err}" }
                        }
                        // The client is fixed at creation; only shown when creating.
                        if editing_id().is_none() {
                            div { class: "form-group",
                                label { class: "form-label", r#for: "proj-client", "Client" }
                                select {
                                    class: "form-input",
                                    id: "proj-client",
                                    value: "{client_id}",
                                    oninput: move |e| client_id.set(e.value()),
                                    option { value: "", "Select a client..." }
                                    if let Some(Ok(clients)) = &*clients_res.read() {
                                        for c in clients.iter().filter(|c| c.active) {
                                            option { value: "{c.id}", "{c.name}" }
                                        }
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
                                // Value is the enum's snake_case `Display`; label via `type_label`,
                                // so the pill and this picker share one source of truth.
                                for t in [
                                    ProjectType::TimeAndMaterials,
                                    ProjectType::FixedFee,
                                    ProjectType::NonBillable,
                                    ProjectType::Retainer,
                                ] {
                                    option { value: "{t}", "{type_label(t)}" }
                                }
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
                                let editing = editing_id();
                                let cid = client_id();
                                let n = name();
                                let pt = project_type();
                                let c = currency();
                                let bk = budget_kind();
                                spawn(async move {
                                    let result = match editing {
                                        Some(id) => {
                                            server_fns::update_project(id.to_string(), n, pt, c, bk).await
                                        }
                                        None => server_fns::create_project(cid, n, pt, c, bk).await,
                                    };
                                    match result {
                                        Ok(_) => {
                                            reset_form();
                                            projects.restart();
                                        }
                                        Err(e) => error.set(Some(e.to_string())),
                                    }
                                });
                            },
                            if editing_id().is_some() { "Save Changes" } else { "Create Project" }
                        }
                    }
                }
            }

            match &*projects.read() {
                Some(Ok(list)) => {
                    let q = query().to_lowercase();
                    let cf = client_filter();
                    let show_all = status_all();
                    let mut items: Vec<Project> = list
                        .iter()
                        .filter(|p| {
                            (show_all || p.active) && (cf.is_empty() || p.client_id.to_string() == cf)
                        })
                        .filter(|p| {
                            if q.is_empty() {
                                return true;
                            }
                            let cn = client_names.get(&p.client_id).cloned().unwrap_or_default();
                            p.name.to_lowercase().contains(&q) || cn.to_lowercase().contains(&q)
                        })
                        .cloned()
                        .collect();
                    // Group by client, ordered by client name then project name.
                    items.sort_by(|a, b| {
                        let an = client_names.get(&a.client_id).cloned().unwrap_or_default();
                        let bn = client_names.get(&b.client_id).cloned().unwrap_or_default();
                        an.to_lowercase()
                            .cmp(&bn.to_lowercase())
                            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                    });
                    let mut groups: Vec<(String, Vec<Project>)> = Vec::new();
                    for p in items {
                        let cn = client_names
                            .get(&p.client_id)
                            .cloned()
                            .unwrap_or_else(|| "Unknown client".to_string());
                        match groups.last_mut() {
                            Some((n, v)) if *n == cn => v.push(p),
                            _ => groups.push((cn, vec![p])),
                        }
                    }
                    if groups.is_empty() {
                        rsx! {
                            div { class: "proj-card",
                                div { class: "proj-empty", "No projects match your filters." }
                            }
                        }
                    } else {
                        rsx! {
                            div { class: "proj-card",
                                div { class: "proj-head",
                                    span { "Project" }
                                    span { class: "text-right", "Budget" }
                                    span { "Status" }
                                    span {}
                                }
                                for (group_name, group) in groups {
                                    div { key: "grp-{group_name}", class: "proj-group", "{group_name}" }
                                    for p in group {
                                        div { class: "proj-row", key: "{p.id}",
                                            div { class: "proj-name-cell",
                                                span { class: "proj-name", "{p.name}" }
                                                span { class: "badge badge-neutral", "{type_label(p.project_type)}" }
                                            }
                                            span { class: "font-mono text-right", "{budget_display(&p)}" }
                                            span {
                                                if p.active {
                                                    span { class: "badge badge-success", "Active" }
                                                } else {
                                                    span { class: "badge badge-neutral", "Inactive" }
                                                }
                                            }
                                            div { class: "flex items-center justify-end gap-3",
                                                Link {
                                                    to: Route::ProjectDetail { id: p.id },
                                                    class: "btn btn-secondary btn-sm",
                                                    "View"
                                                }
                                                if is_manager {
                                                    button {
                                                        class: "btn btn-secondary btn-sm",
                                                        onclick: {
                                                            let p = p.clone();
                                                            move |_| {
                                                                editing_id.set(Some(p.id));
                                                                name.set(p.name.clone());
                                                                project_type.set(p.project_type.to_string());
                                                                currency.set(p.currency.clone());
                                                                budget_kind.set(p.budget_kind.to_string());
                                                                error.set(None);
                                                                show_form.set(true);
                                                            }
                                                        },
                                                        "Edit"
                                                    }
                                                    button {
                                                        class: "btn btn-secondary btn-sm",
                                                        onclick: {
                                                            let id = p.id;
                                                            let next_active = !p.active;
                                                            move |_| {
                                                                spawn(async move {
                                                                    match server_fns::set_project_active(id.to_string(), next_active).await {
                                                                        Ok(_) => projects.restart(),
                                                                        Err(e) => error.set(Some(e.to_string())),
                                                                    }
                                                                });
                                                            }
                                                        },
                                                        if p.active { "Deactivate" } else { "Activate" }
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
                Some(Err(e)) => rsx! { div { class: "alert alert-danger", "{e}" } },
                None => rsx! { div { class: "text-muted text-sm", "Loading..." } },
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
    let users_res = use_resource(|| async move { server_fns::list_users(false).await });

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
                p { class: "text-muted p-5", "Project detail for {id}" }
            }

            // ── Assignments section ─────────────────────────────────────
            div { class: "mt-6",
                div { class: "page-header",
                    h2 { class: "page-title text-xl", "Assignments" }
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
                        div { class: "p-5",
                            h3 { class: "text-sm mb-4 uppercase tracking-wide text-faint", "Assign User" }
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
                            p { class: "text-muted text-sm p-5", "No users assigned yet." }
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
