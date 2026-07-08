use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate};
use dioxus::prelude::*;
use uuid::Uuid;

use crate::models::time_entry::TimeEntry;
use crate::server_fns;

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Day,
    Week,
    Calendar,
}

/// Return the Monday of the ISO week containing `date`.
fn iso_week_monday(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

fn format_decimal_hours(total_minutes: i32) -> String {
    let hours = total_minutes as f64 / 60.0;
    if hours == hours.floor() {
        format!("{}h", hours as i32)
    } else {
        format!("{:.1}h", hours)
    }
}

const DAY_LABELS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

#[component]
pub fn Timesheet() -> Element {
    let today = chrono::Utc::now().date_naive();
    let mut view_mode = use_signal(|| ViewMode::Calendar);
    let mut week_start = use_signal(move || iso_week_monday(today));
    // Which day is selected within the week (0 = Monday .. 6 = Sunday) for Day view
    let mut selected_day_offset = use_signal(|| today.weekday().num_days_from_monday() as i64);

    let entries = use_resource(move || {
        let ws = *week_start.read();
        async move {
            server_fns::list_time_entries(None, None, Some(ws.to_string()), Some(200)).await
        }
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

    let ws = *week_start.read();
    let week_end = ws + Duration::days(6);
    let week_label = format!(
        "{} - {}",
        ws.format("%b %d"),
        week_end.format("%b %d, %Y")
    );

    // Filter entries to this week
    let week_entries: Vec<TimeEntry> = entries
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|es| {
            es.iter()
                .filter(|e| e.spent_date >= ws && e.spent_date <= week_end)
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    // Group entries by day offset (0=Mon .. 6=Sun)
    let mut by_day: [Vec<TimeEntry>; 7] = Default::default();
    for entry in &week_entries {
        let offset = (entry.spent_date - ws).num_days();
        if (0..7).contains(&offset) {
            by_day[offset as usize].push(entry.clone());
        }
    }

    // Daily totals
    let daily_totals: Vec<i32> = by_day.iter().map(|d| d.iter().map(|e| e.minutes).sum()).collect();
    let week_total: i32 = daily_totals.iter().sum();

    // Check if any entries are non-open (already submitted/approved)
    let has_non_open = week_entries.iter().any(|e| e.state != "open");
    let has_open = week_entries.iter().any(|e| e.state == "open");
    let all_submitted_or_approved = !week_entries.is_empty() && !has_open;

    let submit_status = use_signal(|| None::<String>);

    let current_mode = *view_mode.read();
    let sel_offset = *selected_day_offset.read();

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Timesheet" }
            }

            // View mode toggle
            div { style: "display: flex; gap: 0.25rem; margin-bottom: 1rem;",
                button {
                    class: if current_mode == ViewMode::Day { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| view_mode.set(ViewMode::Day),
                    "Day"
                }
                button {
                    class: if current_mode == ViewMode::Week { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| view_mode.set(ViewMode::Week),
                    "Week"
                }
                button {
                    class: if current_mode == ViewMode::Calendar { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| view_mode.set(ViewMode::Calendar),
                    "Calendar"
                }
            }

            // Week navigation
            div { style: "display: flex; align-items: center; gap: 1rem; margin-bottom: 1.5rem;",
                button {
                    class: "btn btn-ghost",
                    onclick: move |_| {
                        week_start.set(ws - Duration::days(7));
                    },
                    "\u{2190} Prev"
                }
                span { style: "font-weight: 500; color: var(--color-text);", "{week_label}" }
                button {
                    class: "btn btn-ghost",
                    onclick: move |_| {
                        week_start.set(ws + Duration::days(7));
                    },
                    "Next \u{2192}"
                }
                button {
                    class: "btn btn-secondary",
                    onclick: move |_| {
                        let t = chrono::Utc::now().date_naive();
                        week_start.set(iso_week_monday(t));
                        selected_day_offset.set(t.weekday().num_days_from_monday() as i64);
                    },
                    "Today"
                }
            }

            // Loading / error states
            match &*entries.read() {
                None => rsx! { div { class: "text-muted text-sm", "Loading..." } },
                Some(Err(e)) => rsx! { div { class: "alert alert-danger", "{e}" } },
                Some(Ok(_)) => rsx! {
                    // Weekly total banner + submit button
                    div { style: "margin-bottom: 1rem; display: flex; align-items: center; gap: 1rem; flex-wrap: wrap;",
                        div { style: "display: flex; align-items: baseline; gap: 0.5rem;",
                            span { class: "text-muted text-sm", "Week total:" }
                            span { class: "text-mono", style: "font-size: 1.25rem; font-weight: 600; color: var(--color-primary);",
                                "{format_decimal_hours(week_total)}"
                            }
                        }

                        if all_submitted_or_approved {
                            span { class: "badge badge-success", "Submitted" }
                        } else if !week_entries.is_empty() && has_open {
                            button {
                                class: "btn btn-accent",
                                disabled: has_non_open && has_open,
                                onclick: {
                                    let ws_str = ws.to_string();
                                    move |_| {
                                        let ws_str = ws_str.clone();
                                        let mut entries = entries;
                                        let mut submit_status = submit_status;
                                        spawn(async move {
                                            match server_fns::submit_week(ws_str).await {
                                                Ok(_) => {
                                                    submit_status.set(None);
                                                    entries.restart();
                                                }
                                                Err(e) => {
                                                    submit_status.set(Some(format!("{e}")));
                                                }
                                            }
                                        });
                                    }
                                },
                                "Submit Week"
                            }
                        }

                        if let Some(err) = &*submit_status.read() {
                            span { style: "color: var(--color-danger); font-size: 0.85rem;", "{err}" }
                        }
                    }

                    match current_mode {
                        ViewMode::Calendar => rsx! {
                            {render_calendar_view(&by_day, &daily_totals, ws, &project_names, &task_names)}
                        },
                        ViewMode::Day => rsx! {
                            {render_day_view(&by_day, daily_totals.as_slice(), ws, sel_offset, selected_day_offset, &project_names, &task_names)}
                        },
                        ViewMode::Week => rsx! {
                            {render_week_view(&week_entries, &daily_totals, ws, &project_names, &task_names)}
                        },
                    }
                },
            }
        }
    }
}

fn render_calendar_view(
    by_day: &[Vec<TimeEntry>; 7],
    daily_totals: &[i32],
    week_start: NaiveDate,
    project_names: &HashMap<Uuid, String>,
    task_names: &HashMap<Uuid, String>,
) -> Element {
    rsx! {
        div { class: "card",
            // Header row
            div { style: "display: grid; grid-template-columns: repeat(7, 1fr); gap: 8px; margin-bottom: 0.5rem;",
                for i in 0..7 {
                    {
                        let day_date = week_start + Duration::days(i as i64);
                        let label = DAY_LABELS[i];
                        rsx! {
                            div { style: "text-align: center; padding: 0.5rem;",
                                div { style: "font-weight: 600; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.06em; color: var(--color-text-muted);",
                                    "{label}"
                                }
                                div { class: "text-mono", style: "font-size: 0.8rem; color: var(--color-text-secondary);",
                                    "{day_date.format(\"%d\")}"
                                }
                            }
                        }
                    }
                }
            }

            // Entry grid
            div { style: "display: grid; grid-template-columns: repeat(7, 1fr); gap: 8px; min-height: 200px;",
                for i in 0..7 {
                    {
                        let day_entries = &by_day[i];
                        let total = daily_totals[i];
                        rsx! {
                            div { style: "display: flex; flex-direction: column; gap: 4px; border-right: 1px solid var(--color-border-light); padding: 0 4px; min-height: 150px;",
                                for entry in day_entries.iter() {
                                    {
                                        let proj = project_names.get(&entry.project_id).cloned().unwrap_or_else(|| "Unknown".into());
                                        let task = task_names.get(&entry.task_id).cloned().unwrap_or_else(|| "\u{2014}".into());
                                        let dur = entry.format_duration();
                                        rsx! {
                                            div { style: "background: var(--color-primary-bg); border: 1px solid var(--color-border); border-radius: var(--radius); padding: 0.5rem; font-size: 0.8rem;",
                                                div { style: "font-weight: 500; color: var(--color-text); margin-bottom: 2px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                                    "{proj}"
                                                }
                                                div { style: "color: var(--color-text-secondary); font-size: 0.75rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                                    "{task}"
                                                }
                                                div { class: "text-mono", style: "color: var(--color-primary); font-size: 0.75rem; margin-top: 2px;",
                                                    "{dur}"
                                                }
                                            }
                                        }
                                    }
                                }
                                // Day total at bottom
                                if total > 0 {
                                    div { style: "margin-top: auto; text-align: center; padding: 0.25rem; border-top: 1px solid var(--color-border-light);",
                                        span { class: "text-mono text-sm", style: "color: var(--color-text-secondary);",
                                            "{format_decimal_hours(total)}"
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

fn render_day_view(
    by_day: &[Vec<TimeEntry>; 7],
    daily_totals: &[i32],
    week_start: NaiveDate,
    selected_offset: i64,
    mut selected_day_offset: Signal<i64>,
    project_names: &HashMap<Uuid, String>,
    task_names: &HashMap<Uuid, String>,
) -> Element {
    let offset = selected_offset.clamp(0, 6) as usize;
    let day_date = week_start + Duration::days(offset as i64);
    let day_entries = &by_day[offset];
    let total = daily_totals[offset];

    rsx! {
        // Day selector tabs
        div { style: "display: flex; gap: 0.25rem; margin-bottom: 1rem;",
            for i in 0i64..7 {
                {
                    let d = week_start + Duration::days(i);
                    let is_sel = i == selected_offset;
                    rsx! {
                        button {
                            class: if is_sel { "btn btn-primary" } else { "btn btn-ghost" },
                            style: "padding: 0.25rem 0.75rem; font-size: 0.8rem;",
                            onclick: move |_| selected_day_offset.set(i),
                            "{DAY_LABELS[i as usize]} {d.format(\"%d\")}"
                        }
                    }
                }
            }
        }

        div { class: "card",
            h3 { style: "margin-bottom: 1rem; color: var(--color-text);",
                "{day_date.format(\"%A, %B %d, %Y\")}"
            }

            if day_entries.is_empty() {
                div { class: "text-muted text-sm", style: "padding: 2rem; text-align: center;",
                    "No entries for this day."
                }
            } else {
                div { class: "table-container",
                    table {
                        thead {
                            tr {
                                th { "Project" }
                                th { "Task" }
                                th { "Duration" }
                                th { "Notes" }
                                th { "Billable" }
                            }
                        }
                        tbody {
                            for entry in day_entries.iter() {
                                {
                                    let proj = project_names.get(&entry.project_id).cloned().unwrap_or_else(|| entry.project_id.to_string());
                                    let task = task_names.get(&entry.task_id).cloned().unwrap_or_else(|| "\u{2014}".into());
                                    rsx! {
                                        tr {
                                            td { "{proj}" }
                                            td { "{task}" }
                                            td { class: "text-mono",
                                                if entry.is_running {
                                                    span { class: "badge badge-success", "Running" }
                                                } else {
                                                    "{entry.format_duration()}"
                                                }
                                            }
                                            td { "{entry.notes.as_deref().unwrap_or(\"-\")}" }
                                            td {
                                                if entry.billable {
                                                    span { class: "badge badge-info", "Billable" }
                                                } else {
                                                    span { class: "badge badge-neutral", "No" }
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

            div { style: "margin-top: 1rem; text-align: right; padding: 0.5rem;",
                span { class: "text-muted text-sm", "Day total: " }
                span { class: "text-mono", style: "font-weight: 600; color: var(--color-primary);",
                    "{format_decimal_hours(total)}"
                }
            }
        }
    }
}

fn render_week_view(
    entries: &[TimeEntry],
    daily_totals: &[i32],
    week_start: NaiveDate,
    project_names: &HashMap<Uuid, String>,
    task_names: &HashMap<Uuid, String>,
) -> Element {
    // Group by (project_id, task_id) preserving insertion order
    let mut row_keys: Vec<(Uuid, Uuid)> = Vec::new();
    let mut row_map: HashMap<(Uuid, Uuid), [i32; 7]> = HashMap::new();

    for entry in entries {
        let key = (entry.project_id, entry.task_id);
        let offset = (entry.spent_date - week_start).num_days();
        if !(0..7).contains(&offset) {
            continue;
        }
        let row = row_map.entry(key).or_insert_with(|| {
            row_keys.push(key);
            [0i32; 7]
        });
        row[offset as usize] += entry.minutes;
    }

    rsx! {
        div { class: "card",
            div { class: "table-container",
                table {
                    thead {
                        tr {
                            th { "Project" }
                            th { "Task" }
                            for i in 0..7 {
                                {
                                    let d = week_start + Duration::days(i as i64);
                                    rsx! {
                                        th { style: "text-align: center; min-width: 60px;",
                                            "{DAY_LABELS[i]}"
                                            br {}
                                            span { class: "text-mono", style: "font-size: 0.75rem;", "{d.format(\"%d\")}" }
                                        }
                                    }
                                }
                            }
                            th { style: "text-align: center;", "Total" }
                        }
                    }
                    tbody {
                        for key in row_keys.iter() {
                            {
                                let (pid, tid) = *key;
                                let proj = project_names.get(&pid).cloned().unwrap_or_else(|| pid.to_string());
                                let task = task_names.get(&tid).cloned().unwrap_or_else(|| "\u{2014}".into());
                                let row = row_map.get(key).copied().unwrap_or([0; 7]);
                                let row_total: i32 = row.iter().sum();
                                rsx! {
                                    tr {
                                        td { "{proj}" }
                                        td { "{task}" }
                                        for i in 0..7 {
                                            {
                                                let mins = row[i];
                                                rsx! {
                                                    td { class: "text-mono", style: "text-align: center;",
                                                        if mins > 0 {
                                                            "{format_decimal_hours(mins)}"
                                                        } else {
                                                            span { class: "text-muted", "\u{2014}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        td { class: "text-mono", style: "text-align: center; font-weight: 600;",
                                            "{format_decimal_hours(row_total)}"
                                        }
                                    }
                                }
                            }
                        }
                        // Totals row
                        tr { style: "border-top: 2px solid var(--color-border); font-weight: 600;",
                            td { colspan: "2", "Daily Totals" }
                            for i in 0..7 {
                                {
                                    let t = daily_totals[i];
                                    rsx! {
                                        td { class: "text-mono", style: "text-align: center;",
                                            if t > 0 {
                                                "{format_decimal_hours(t)}"
                                            } else {
                                                span { class: "text-muted", "\u{2014}" }
                                            }
                                        }
                                    }
                                }
                            }
                            td { class: "text-mono", style: "text-align: center; color: var(--color-primary);",
                                "{format_decimal_hours(daily_totals.iter().sum::<i32>())}"
                            }
                        }
                    }
                }
            }
        }
    }
}
