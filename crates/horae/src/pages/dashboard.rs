use std::collections::HashMap;

use chrono::Datelike;
use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::timer_widget::TimerWidget;
use crate::server_fns;

#[component]
pub fn Dashboard() -> Element {
    // Fetch enough entries to cover the current week (plus recent for the table)
    let entries = use_resource(|| async move {
        // Fetch entries from the start of the current ISO week
        let today = chrono::Utc::now().date_naive();
        let weekday = today.weekday().num_days_from_monday();
        let week_start = today - chrono::Duration::days(weekday as i64);
        server_fns::list_time_entries(None, None, Some(week_start.to_string()), None, Some(200))
            .await
    });
    let projects = use_resource(|| async move { server_fns::list_projects(None, false).await });

    let project_names: HashMap<Uuid, String> = projects
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|ps| ps.iter().map(|p| (p.id, p.name.clone())).collect())
        .unwrap_or_default();

    // Compute stats from entries
    let (hours_this_week, unbilled_hours) = entries
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|es| {
            let today = chrono::Utc::now().date_naive();
            let weekday = today.weekday().num_days_from_monday();
            let week_start = today - chrono::Duration::days(weekday as i64);

            let mut week_minutes: i64 = 0;
            let mut unbilled_minutes: i64 = 0;

            for e in es.iter() {
                if e.spent_date >= week_start {
                    week_minutes += e.minutes as i64;
                }
                if e.billable && e.invoice_id.is_none() {
                    unbilled_minutes += e.minutes as i64;
                }
            }

            let week_hours = week_minutes as f64 / 60.0;
            let unbilled = unbilled_minutes as f64 / 60.0;
            (format!("{week_hours:.1}"), format!("{unbilled:.1}"))
        })
        .unwrap_or_else(|| ("\u{2014}".into(), "\u{2014}".into()));

    let active_project_count = projects
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|ps| ps.iter().filter(|p| p.active).count().to_string())
        .unwrap_or_else(|| "\u{2014}".into());

    // Recent entries for the table (last 10)
    let recent_entries: Vec<_> = entries
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|es| es.iter().take(10).cloned().collect())
        .unwrap_or_default();

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Dashboard" }
            }

            div { class: "grid-stats",
                div { class: "stat-card",
                    div { class: "stat-label", "Hours This Week" }
                    div { class: "stat-value text-mono", "{hours_this_week}" }
                }
                div { class: "stat-card",
                    div { class: "stat-label", "Active Projects" }
                    div { class: "stat-value text-mono", "{active_project_count}" }
                }
                div { class: "stat-card",
                    div { class: "stat-label", "Unbilled Hours" }
                    div { class: "stat-value text-mono", "{unbilled_hours}" }
                }
            }

            TimerWidget {}

            div { class: "card mt-4",
                h2 { class: "card-title", "Recent Time Entries" }
                if recent_entries.is_empty() {
                    div { class: "text-muted text-sm", style: "padding: 1rem;",
                        "No time entries yet. Use the timer or log time manually."
                    }
                } else {
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
                                for entry in recent_entries.iter() {
                                    tr { key: "{entry.id}",
                                        td { class: "text-mono", "{entry.spent_date}" }
                                        td {
                                            {project_names.get(&entry.project_id)
                                                .cloned()
                                                .unwrap_or_else(|| entry.project_id.to_string())}
                                        }
                                        td { class: "text-mono",
                                            if entry.is_running {
                                                span { class: "badge badge-success", "Running" }
                                            } else {
                                                "{entry.format_duration()}"
                                            }
                                        }
                                        td { "{entry.notes.as_deref().unwrap_or(\"-\")}" }
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
