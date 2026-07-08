use std::collections::HashMap;

use dioxus::prelude::*;
use uuid::Uuid;

use crate::server_fns;
use crate::components::timer_widget::TimerWidget;

#[component]
pub fn Dashboard() -> Element {
    let entries = use_resource(|| async move {
        server_fns::list_time_entries(None, None, None, Some(10)).await
    });
    let projects = use_resource(|| async move { server_fns::list_projects(None, None).await });

    let project_names: HashMap<Uuid, String> = projects
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|ps| ps.iter().map(|p| (p.id, p.name.clone())).collect())
        .unwrap_or_default();

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Dashboard" }
            }

            div { class: "grid-stats",
                div { class: "stat-card",
                    div { class: "stat-label", "Hours This Week" }
                    div { class: "stat-value", "—" }
                }
                div { class: "stat-card",
                    div { class: "stat-label", "Active Projects" }
                    div { class: "stat-value", "—" }
                }
                div { class: "stat-card",
                    div { class: "stat-label", "Unbilled Hours" }
                    div { class: "stat-value", "—" }
                }
            }

            TimerWidget {}

            div { class: "card mt-4",
                h2 { class: "card-title", "Recent Time Entries" }
                match &*entries.read() {
                    Some(Ok(entries)) => rsx! {
                        div { class: "table-container",
                            table {
                                thead {
                                    tr {
                                        th { "Date" }
                                        th { "Project" }
                                        th { "Duration" }
                                        th { "Notes" }
                                    }
                                }
                                tbody {
                                    for entry in entries.iter() {
                                        tr { key: "{entry.id}",
                                            td { class: "text-mono", "{entry.started_at.format(\"%Y-%m-%d\")}" }
                                            td {
                                                {project_names.get(&entry.project_id)
                                                    .cloned()
                                                    .unwrap_or_else(|| entry.project_id.to_string())}
                                            }
                                            td { class: "text-mono", "{entry.duration_seconds / 3600}h {(entry.duration_seconds % 3600) / 60}m" }
                                            td { "{entry.notes.as_deref().unwrap_or(\"-\")}" }
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
