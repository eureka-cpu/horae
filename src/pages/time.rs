use std::collections::HashMap;

use dioxus::prelude::*;
use uuid::Uuid;

use crate::server_fns;
use crate::components::timer_widget::TimerWidget;

#[component]
pub fn TimeList() -> Element {
    let entries = use_resource(|| async move {
        server_fns::list_time_entries(None, None, None, Some(50)).await
    });
    let projects = use_resource(|| async move { server_fns::list_projects(None, None).await });
    let tasks = use_resource(|| async move { server_fns::list_tasks(None).await });

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

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Time Entries" }
                div { class: "page-actions",
                    button { class: "btn btn-primary", "Log Time" }
                }
            }

            TimerWidget {}

            div { class: "card mt-4",
                match &*entries.read() {
                    Some(Ok(entries)) => rsx! {
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
                                    for entry in entries.iter() {
                                        tr { key: "{entry.id}",
                                            td { "{entry.started_at.format(\"%Y-%m-%d\")}" }
                                            td {
                                                {project_names.get(&entry.project_id)
                                                    .cloned()
                                                    .unwrap_or_else(|| entry.project_id.to_string())}
                                            }
                                            td {
                                                {entry.task_id
                                                    .and_then(|id| task_names.get(&id).cloned())
                                                    .unwrap_or_else(|| "—".into())}
                                            }
                                            td {
                                                if entry.is_running() {
                                                    span { class: "badge badge-success", "Running" }
                                                } else {
                                                    "{entry.duration_seconds / 3600}h {(entry.duration_seconds % 3600) / 60}m"
                                                }
                                            }
                                            td {
                                                if entry.is_billable {
                                                    span { class: "badge badge-info", "Billable" }
                                                } else {
                                                    span { class: "badge badge-neutral", "No" }
                                                }
                                            }
                                            td { "{entry.notes.as_deref().unwrap_or(\"-\")}" }
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
