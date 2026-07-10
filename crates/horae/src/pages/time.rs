use std::collections::HashMap;

use dioxus::prelude::*;
use tracing::error;
use uuid::Uuid;

use crate::components::timer_widget::TimerWidget;
use crate::server_fns;

#[component]
pub fn TimeList() -> Element {
    let mut entries = use_resource(|| async move {
        server_fns::list_time_entries(None, None, None, None, Some(50)).await
    });
    let projects = use_resource(|| async move { server_fns::list_projects(None, None).await });
    let tasks = use_resource(|| async move { server_fns::list_tasks().await });

    // "Log Time" form visibility
    let mut show_form = use_signal(|| false);

    // Form state
    let mut form_project = use_signal(String::new);
    let mut form_task = use_signal(String::new);
    let mut form_date = use_signal(|| chrono::Utc::now().date_naive().to_string());
    let mut form_minutes = use_signal(|| String::from("0"));
    let mut form_notes = use_signal(String::new);
    let mut form_billable = use_signal(|| false);
    let mut form_error = use_signal(|| Option::<String>::None);

    // Edit state: which entry is being edited
    let mut editing_id = use_signal(|| Option::<Uuid>::None);
    let mut edit_minutes = use_signal(String::new);
    let mut edit_notes = use_signal(String::new);
    let mut edit_billable = use_signal(|| false);

    let project_names: HashMap<Uuid, String> = projects
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|ps| ps.iter().map(|p| (p.id, p.name.clone())).collect())
        .unwrap_or_default();

    let task_names: HashMap<Uuid, String> = tasks
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|ts| ts.iter().map(|t| (t.id, t.name.clone())).collect())
        .unwrap_or_default();

    let handle_create = move |e: Event<FormData>| {
        e.prevent_default();
        let project_id = form_project.read().clone();
        let task_id = form_task.read().clone();
        let date = form_date.read().clone();
        let minutes_str = form_minutes.read().clone();
        let notes = form_notes.read().clone();
        let billable = *form_billable.read();

        spawn(async move {
            let minutes: i32 = match minutes_str.parse() {
                Ok(m) => m,
                Err(_) => {
                    form_error.set(Some("Invalid minutes value".into()));
                    return;
                }
            };

            if project_id.is_empty() || task_id.is_empty() {
                form_error.set(Some("Project and task are required".into()));
                return;
            }

            let notes_opt = if notes.is_empty() { None } else { Some(notes) };

            match server_fns::create_time_entry(
                project_id, task_id, date, minutes, notes_opt, billable,
            )
            .await
            {
                Ok(_) => {
                    show_form.set(false);
                    form_error.set(None);
                    form_minutes.set("0".into());
                    form_notes.set(String::new());
                    form_billable.set(false);
                    entries.restart();
                }
                Err(e) => {
                    form_error.set(Some(format!("{e}")));
                }
            }
        });
    };

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Time Entries" }
                div { class: "page-actions",
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| show_form.set(!show_form()),
                        if *show_form.read() { "Cancel" } else { "Log Time" }
                    }
                }
            }

            TimerWidget {}

            // Log Time form
            if *show_form.read() {
                div { class: "card mt-4",
                    h2 { class: "card-title", "Log Time Entry" }
                    if let Some(err) = form_error.read().as_ref() {
                        div { class: "alert alert-danger", "{err}" }
                    }
                    form { onsubmit: handle_create,
                        div { class: "form-grid", style: "display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;",
                            div { class: "form-group",
                                label { class: "form-label", "Project" }
                                select {
                                    class: "form-input",
                                    value: "{form_project}",
                                    oninput: move |e| form_project.set(e.value()),
                                    option { value: "", "Select project..." }
                                    {projects.read().as_ref().and_then(|r| r.as_ref().ok()).map(|ps| {
                                        rsx! {
                                            for p in ps.iter() {
                                                option { value: "{p.id}", "{p.name}" }
                                            }
                                        }
                                    })}
                                }
                            }
                            div { class: "form-group",
                                label { class: "form-label", "Task" }
                                select {
                                    class: "form-input",
                                    value: "{form_task}",
                                    oninput: move |e| form_task.set(e.value()),
                                    option { value: "", "Select task..." }
                                    {tasks.read().as_ref().and_then(|r| r.as_ref().ok()).map(|ts| {
                                        rsx! {
                                            for t in ts.iter() {
                                                option { value: "{t.id}", "{t.name}" }
                                            }
                                        }
                                    })}
                                }
                            }
                            div { class: "form-group",
                                label { class: "form-label", "Date" }
                                input {
                                    class: "form-input",
                                    r#type: "date",
                                    value: "{form_date}",
                                    oninput: move |e| form_date.set(e.value()),
                                }
                            }
                            div { class: "form-group",
                                label { class: "form-label", "Minutes" }
                                input {
                                    class: "form-input",
                                    r#type: "number",
                                    min: "0",
                                    value: "{form_minutes}",
                                    oninput: move |e| form_minutes.set(e.value()),
                                }
                            }
                        }
                        div { class: "form-group", style: "margin-top: 1rem;",
                            label { class: "form-label", "Notes" }
                            input {
                                class: "form-input",
                                r#type: "text",
                                placeholder: "What did you work on?",
                                value: "{form_notes}",
                                oninput: move |e| form_notes.set(e.value()),
                            }
                        }
                        div { class: "form-group", style: "margin-top: 0.75rem; display: flex; align-items: center; gap: 0.5rem;",
                            input {
                                r#type: "checkbox",
                                checked: *form_billable.read(),
                                oninput: move |e| {
                                    form_billable.set(e.value() == "true");
                                },
                            }
                            label { class: "form-label", style: "margin: 0;", "Billable" }
                        }
                        div { style: "margin-top: 1rem;",
                            button { class: "btn btn-primary", r#type: "submit", "Save Entry" }
                        }
                    }
                }
            }

            div { class: "card mt-4",
                match &*entries.read() {
                    Some(Ok(entry_list)) => {
                        let entry_list = entry_list.clone();
                        rsx! {
                            div { class: "table-container",
                                table {
                                    thead {
                                        tr {
                                            th { "Date" }
                                            th { "Project" }
                                            th { "Task" }
                                            th { "Duration" }
                                            th { "Billable" }
                                            th { "Notes" }
                                            th { "Actions" }
                                        }
                                    }
                                    tbody {
                                        for entry in entry_list.iter() {
                                            {
                                                let eid = entry.id;
                                                let is_editing = *editing_id.read() == Some(eid);
                                                let is_open = entry.state == horae_core::types::EntryState::Open;
                                                let entry_id_str = eid.to_string();
                                                let entry_minutes = entry.minutes;
                                                let entry_notes = entry.notes.clone().unwrap_or_default();
                                                let entry_billable = entry.billable;

                                                rsx! {
                                                    tr { key: "{eid}",
                                                        td { class: "text-mono", "{entry.spent_date}" }
                                                        td {
                                                            {project_names.get(&entry.project_id)
                                                                .cloned()
                                                                .unwrap_or_else(|| entry.project_id.to_string())}
                                                        }
                                                        td {
                                                            {task_names.get(&entry.task_id)
                                                                .cloned()
                                                                .unwrap_or_else(|| "\u{2014}".into())}
                                                        }
                                                        td { class: "text-mono",
                                                            if entry.is_running {
                                                                span { class: "badge badge-success", "Running" }
                                                            } else if is_editing {
                                                                input {
                                                                    class: "form-input",
                                                                    r#type: "number",
                                                                    min: "0",
                                                                    style: "width: 5rem;",
                                                                    value: "{edit_minutes}",
                                                                    oninput: move |e| edit_minutes.set(e.value()),
                                                                }
                                                            } else {
                                                                "{entry.format_duration()}"
                                                            }
                                                        }
                                                        td {
                                                            if is_editing {
                                                                input {
                                                                    r#type: "checkbox",
                                                                    checked: *edit_billable.read(),
                                                                    oninput: move |e| {
                                                                        edit_billable.set(e.value() == "true");
                                                                    },
                                                                }
                                                            } else if entry.billable {
                                                                span { class: "badge badge-info", "Billable" }
                                                            } else {
                                                                span { class: "badge badge-neutral", "No" }
                                                            }
                                                        }
                                                        td {
                                                            if is_editing {
                                                                input {
                                                                    class: "form-input",
                                                                    r#type: "text",
                                                                    value: "{edit_notes}",
                                                                    oninput: move |e| edit_notes.set(e.value()),
                                                                }
                                                            } else {
                                                                "{entry.notes.as_deref().unwrap_or(\"-\")}"
                                                            }
                                                        }
                                                        td { style: "display: flex; gap: 0.25rem;",
                                                            if is_editing {
                                                                button {
                                                                    class: "btn btn-primary btn-sm",
                                                                    onclick: {
                                                                        let entry_id_str = entry_id_str.clone();
                                                                        move |_| {
                                                                            let eid = entry_id_str.clone();
                                                                            let mins = edit_minutes.read().clone();
                                                                            let notes = edit_notes.read().clone();
                                                                            let billable = *edit_billable.read();
                                                                            spawn(async move {
                                                                                let minutes: i32 = mins.parse().unwrap_or(0);
                                                                                let notes_opt = if notes.is_empty() { None } else { Some(notes) };
                                                                                match server_fns::update_time_entry(eid, minutes, notes_opt, billable).await {
                                                                                    Ok(_) => {
                                                                                        editing_id.set(None);
                                                                                        entries.restart();
                                                                                    }
                                                                                    Err(e) => {
                                                                                        error!("Update error: {e}");
                                                                                    }
                                                                                }
                                                                            });
                                                                        }
                                                                    },
                                                                    "Save"
                                                                }
                                                                button {
                                                                    class: "btn btn-secondary btn-sm",
                                                                    onclick: move |_| editing_id.set(None),
                                                                    "Cancel"
                                                                }
                                                            } else {
                                                                if is_open && !entry.is_running {
                                                                    button {
                                                                        class: "btn btn-secondary btn-sm",
                                                                        onclick: move |_| {
                                                                            editing_id.set(Some(eid));
                                                                            edit_minutes.set(entry_minutes.to_string());
                                                                            edit_notes.set(entry_notes.clone());
                                                                            edit_billable.set(entry_billable);
                                                                        },
                                                                        "Edit"
                                                                    }
                                                                    button {
                                                                        class: "btn btn-danger btn-sm",
                                                                        onclick: {
                                                                            let entry_id_str = entry_id_str.clone();
                                                                            move |_| {
                                                                                let eid = entry_id_str.clone();
                                                                                spawn(async move {
                                                                                    match server_fns::delete_time_entry(eid).await {
                                                                                        Ok(_) => {
                                                                                            entries.restart();
                                                                                        }
                                                                                        Err(e) => {
                                                                                            error!("Delete error: {e}");
                                                                                        }
                                                                                    }
                                                                                });
                                                                            }
                                                                        },
                                                                        "Delete"
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
                    },
                    Some(Err(e)) => rsx! { div { class: "alert alert-danger", "{e}" } },
                    None => rsx! { div { class: "text-muted text-sm", "Loading..." } },
                }
            }
        }
    }
}
