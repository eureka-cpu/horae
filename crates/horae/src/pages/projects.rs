use dioxus::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;

use crate::components::combobox::{ComboOption, Combobox};
use crate::components::menu::{Menu, MenuDivider, MenuItem};
use crate::models::{Client, Project};
use crate::route::Route;
use crate::server_fns;
use horae_core::money::format_cents;
use horae_core::types::{BudgetKind, ProjectType};

fn hours(minutes: i64) -> String {
    format!(
        "{}h",
        horae_core::duration::format_decimal(minutes.max(0) as u32)
    )
}

/// Budget / Spent / Budget-remaining for one row, expressed in the project's own
/// budget unit (money for amount budgets, hours for hours budgets).
struct RowSpend {
    budget: String,
    recurring: bool,
    spent: String,
    remaining: String,
    /// Consumption for the progress bar, clamped 0..=100 (None = no budget set).
    pct: Option<u8>,
    /// "(NN%)" shown next to Budget remaining (None = no budget set).
    pct_label: Option<String>,
}

fn row_spend(p: &Project, spent_minutes: i64, spent_cents: i64) -> RowSpend {
    let cur = p.currency.trim();
    let recurring = matches!(p.project_type, ProjectType::Retainer);
    let pct_of = |spent: i64, budget: i64| -> (Option<u8>, Option<String>) {
        if budget > 0 {
            let raw = (spent as f64 / budget as f64 * 100.0).round() as i64;
            (
                Some(raw.clamp(0, 100) as u8),
                Some(format!("({}%)", raw.max(0))),
            )
        } else {
            (None, None)
        }
    };
    match p.budget_kind {
        BudgetKind::Amount => {
            let budget = p.budget_amount_cents.unwrap_or(0);
            let (pct, pct_label) = pct_of(spent_cents, budget);
            RowSpend {
                budget: format_cents(budget, cur),
                recurring,
                spent: format_cents(spent_cents, cur),
                remaining: format_cents(budget - spent_cents, cur),
                pct,
                pct_label,
            }
        }
        BudgetKind::Hours => {
            let budget = p.budget_minutes.unwrap_or(0);
            let (pct, pct_label) = pct_of(spent_minutes, budget);
            RowSpend {
                budget: hours(budget),
                recurring,
                spent: hours(spent_minutes),
                remaining: hours(budget - spent_minutes),
                pct,
                pct_label,
            }
        }
        BudgetKind::None => RowSpend {
            budget: "—".to_string(),
            recurring,
            spent: format_cents(spent_cents, cur),
            remaining: "—".to_string(),
            pct: None,
            pct_label: None,
        },
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
    let spend_res = use_resource(|| async move { server_fns::list_project_spend().await });

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
    // Status scope: "active" | "budgeted" (has a budget) | "archived" (inactive).
    let mut scope = use_signal(|| "active".to_string());
    let mut client_filter = use_signal(String::new);
    // Export modal: open state + chosen scope and format.
    let mut export_open = use_signal(|| false);
    let mut export_scope = use_signal(|| "active".to_string());
    let mut export_fmt = use_signal(|| "csv".to_string());

    let is_manager = match &*me.read() {
        Some(Ok(user)) => user.is_manager_or_above(),
        _ => false,
    };

    let client_names: HashMap<Uuid, String> = match &*clients_res.read() {
        Some(Ok(cs)) => cs.iter().map(|c| (c.id, c.name.clone())).collect(),
        _ => HashMap::new(),
    };
    // project_id -> (spent_minutes, spent_cents); missing = no tracked time yet.
    let spend_map: HashMap<Uuid, (i64, i64)> = match &*spend_res.read() {
        Some(Ok(v)) => v
            .iter()
            .map(|s| (s.project_id, (s.spent_minutes, s.spent_cents)))
            .collect(),
        _ => HashMap::new(),
    };
    let (active_count, budgeted_count, archived_count) = match &*projects.read() {
        Some(Ok(list)) => (
            list.iter().filter(|p| p.active).count(),
            list.iter()
                .filter(|p| p.active && p.budget_kind != BudgetKind::None)
                .count(),
            list.iter().filter(|p| !p.active).count(),
        ),
        _ => (0, 0, 0),
    };
    let scope_label = match scope().as_str() {
        "budgeted" => format!("Budgeted projects ({budgeted_count})"),
        "archived" => format!("Archived projects ({archived_count})"),
        _ => format!("Active projects ({active_count})"),
    };
    // Client options for the filter combobox, grouped Active / Archived.
    let client_options: Vec<ComboOption> = match &*clients_res.read() {
        Some(Ok(cs)) => {
            let mut active: Vec<&Client> = cs.iter().filter(|c| c.active).collect();
            let mut archived: Vec<&Client> = cs.iter().filter(|c| !c.active).collect();
            let by_name =
                |a: &&Client, b: &&Client| a.name.to_lowercase().cmp(&b.name.to_lowercase());
            active.sort_by(by_name);
            archived.sort_by(by_name);
            active
                .into_iter()
                .map(|c| ComboOption::grouped(c.id.to_string(), c.name.clone(), "Active clients"))
                .chain(archived.into_iter().map(|c| {
                    ComboOption::grouped(c.id.to_string(), c.name.clone(), "Archived clients")
                }))
                .collect()
        }
        _ => Vec::new(),
    };

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
                button {
                    class: "btn btn-secondary btn-sm ml-auto",
                    onclick: move |_| export_open.set(true),
                    "Export"
                }
                div { class: "proj-search",
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
                Menu { label: "{scope_label}",
                    MenuItem {
                        selected: scope() == "active",
                        onclick: move |_| scope.set("active".to_string()),
                        "Active projects ({active_count})"
                    }
                    MenuItem {
                        selected: scope() == "budgeted",
                        onclick: move |_| scope.set("budgeted".to_string()),
                        "Budgeted projects ({budgeted_count})"
                    }
                    MenuItem {
                        selected: scope() == "archived",
                        onclick: move |_| scope.set("archived".to_string()),
                        "Archived projects ({archived_count})"
                    }
                }
                div { class: "flex-1" }
                Combobox {
                    options: client_options,
                    value: client_filter(),
                    placeholder: "Filter by client",
                    all_label: "All clients",
                    onselect: move |v| client_filter.set(v),
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
                                // Value is the enum's snake_case `Display`; label via ProjectType::label,
                                // so the pill and this picker share one source of truth.
                                for t in [
                                    ProjectType::TimeAndMaterials,
                                    ProjectType::FixedFee,
                                    ProjectType::NonBillable,
                                    ProjectType::Retainer,
                                ] {
                                    option { value: "{t}", "{t.label()}" }
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
                    let sc = scope();
                    let mut items: Vec<Project> = list
                        .iter()
                        .filter(|p| {
                            let scope_ok = match sc.as_str() {
                                "budgeted" => p.active && p.budget_kind != BudgetKind::None,
                                "archived" => !p.active,
                                _ => p.active,
                            };
                            scope_ok && (cf.is_empty() || p.client_id.to_string() == cf)
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
                                    span { "Client" }
                                    span { class: "text-right", "Budget" }
                                    span { class: "text-right", "⚑ Scheduled" }
                                    span { class: "text-right", "Delta" }
                                    span { class: "text-right", "Spent" }
                                    span { class: "text-right", "Budget remaining" }
                                    span {}
                                }
                                for (group_name, group) in groups {
                                    div { key: "grp-{group_name}", class: "proj-group", "{group_name}" }
                                    for p in group {
                                        {
                                            let (sm, sc) = spend_map.get(&p.id).copied().unwrap_or((0, 0));
                                            let rs = row_spend(&p, sm, sc);
                                            let pname = match &p.code {
                                                Some(c) => format!("[{c}] {}", p.name),
                                                None => p.name.clone(),
                                            };
                                            rsx! {
                                            div { class: "proj-row", key: "{p.id}",
                                            div { class: "flex items-center gap-3 min-w-0",
                                                Link {
                                                    to: Route::ProjectDetail { id: p.id },
                                                    class: "font-semibold proj-namelink",
                                                    "{pname}"
                                                }
                                                span { class: "badge badge-neutral", "{p.project_type.label()}" }
                                                if !p.active {
                                                    span { class: "badge badge-neutral", "Inactive" }
                                                }
                                            }
                                            div { class: "flex items-center justify-end gap-2 font-mono",
                                                span { class: "whitespace-nowrap", "{rs.budget}" }
                                                if rs.recurring {
                                                    span { class: "text-faint", "⟳" }
                                                }
                                            }
                                            span { class: "font-mono text-right text-faint", "–" }
                                            span { class: "font-mono text-right text-faint", "–" }
                                            div { class: "flex items-center justify-end gap-3 font-mono",
                                                span { class: "whitespace-nowrap", "{rs.spent}" }
                                                if let Some(pct) = rs.pct {
                                                    div { class: "proj-bar",
                                                        div { class: "proj-bar-fill", style: "width: {pct}%" }
                                                    }
                                                }
                                            }
                                            div { class: "flex items-baseline justify-end gap-2 font-mono",
                                                span { class: "whitespace-nowrap", "{rs.remaining}" }
                                                if let Some(lbl) = rs.pct_label.clone() {
                                                    span { class: "text-faint", "{lbl}" }
                                                }
                                            }
                                            div { class: "flex justify-end",
                                                if is_manager {
                                                    Menu { label: "Actions", align_right: true,
                                                        MenuItem {
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
                                                        MenuDivider {}
                                                        MenuItem {
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
                                                            if p.active { "Archive" } else { "Unarchive" }
                                                        }
                                                    }
                                                } else {
                                                    Link {
                                                        to: Route::ProjectDetail { id: p.id },
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
                    }
                }
                Some(Err(e)) => rsx! { div { class: "alert alert-danger", "{e}" } },
                None => rsx! { div { class: "text-muted text-sm", "Loading..." } },
            }

            if export_open() {
                {
                    let url = format!(
                        "/api/projects/export/{}?scope={}",
                        export_fmt(),
                        export_scope()
                    );
                    rsx! {
                        div {
                            class: "modal-overlay",
                            onclick: move |_| export_open.set(false),
                            div {
                                class: "modal",
                                onclick: move |e| e.stop_propagation(),
                                div { class: "modal-title", "Export projects" }
                                div { class: "modal-body",
                                    div { class: "modal-label", "Which projects?" }
                                    div { class: "seg-row",
                                        for (val , lbl) in [("active", "Active"), ("budgeted", "Budgeted"), ("archived", "Archived")] {
                                            button {
                                                class: if export_scope() == val { "seg-btn selected" } else { "seg-btn" },
                                                onclick: move |_| export_scope.set(val.to_string()),
                                                "{lbl}"
                                            }
                                        }
                                    }
                                    div { class: "modal-label", "Format" }
                                    div { class: "seg-row",
                                        for (val , lbl) in [("csv", "CSV"), ("xlsx", "Excel")] {
                                            button {
                                                class: if export_fmt() == val { "seg-btn selected" } else { "seg-btn" },
                                                onclick: move |_| export_fmt.set(val.to_string()),
                                                "{lbl}"
                                            }
                                        }
                                    }
                                    div { class: "modal-actions",
                                        a {
                                            class: "btn btn-primary",
                                            href: "{url}",
                                            onclick: move |_| export_open.set(false),
                                            "Export projects"
                                        }
                                        button {
                                            class: "btn btn-secondary",
                                            onclick: move |_| export_open.set(false),
                                            "Cancel"
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
