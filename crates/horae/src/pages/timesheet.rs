use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate};
use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::controls::Segmented;
use crate::models::time_entry::TimeEntry;
use crate::route::Route;
use crate::server_fns;

/// `H:MM` clock format from integer minutes (the design's cell/total format).
fn format_hm(total_minutes: i32) -> String {
    format!("{}:{:02}", total_minutes / 60, total_minutes % 60)
}

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
    let mut view_mode = use_signal(|| ViewMode::Week);
    let mut week_start = use_signal(move || iso_week_monday(today));
    // Which day is selected within the week (0 = Monday .. 6 = Sunday) for Day view
    let mut selected_day_offset = use_signal(|| today.weekday().num_days_from_monday() as i64);

    let entries = use_resource(move || {
        let ws = *week_start.read();
        async move {
            let we = ws + chrono::Duration::days(6);
            server_fns::list_time_entries(
                None,
                None,
                Some(ws.to_string()),
                Some(we.to_string()),
                Some(200),
            )
            .await
        }
    });
    let projects = use_resource(|| async move { server_fns::list_projects(None, false).await });
    let tasks = use_resource(|| async move { server_fns::list_tasks().await });
    let clients = use_resource(|| async move { server_fns::list_clients(true).await });

    let project_names: HashMap<Uuid, String> = projects
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|ps| ps.iter().map(|p| (p.id, p.name.clone())).collect())
        .unwrap_or_default();

    // project_id -> (client name, project currency), for the calendar event's
    // "Client · CUR" line.
    let project_client: HashMap<Uuid, (String, String)> = {
        let client_names: HashMap<Uuid, String> = clients
            .read()
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .map(|cs| cs.iter().map(|c| (c.id, c.name.clone())).collect())
            .unwrap_or_default();
        projects
            .read()
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .map(|ps| {
                ps.iter()
                    .map(|p| {
                        let name = client_names.get(&p.client_id).cloned().unwrap_or_default();
                        (p.id, (name, p.currency.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    };

    let task_names: HashMap<Uuid, String> = tasks
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|ts| ts.iter().map(|t| (t.id, t.name.clone())).collect())
        .unwrap_or_default();

    let ws = *week_start.read();
    let week_end = ws + Duration::days(6);

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
    let daily_totals: Vec<i32> = by_day
        .iter()
        .map(|d| d.iter().map(|e| e.minutes).sum())
        .collect();
    let week_total: i32 = daily_totals.iter().sum();

    // Check if any entries are non-open (already submitted/approved)
    let has_non_open = week_entries
        .iter()
        .any(|e| e.state != horae_core::types::EntryState::Open);
    let has_open = week_entries
        .iter()
        .any(|e| e.state == horae_core::types::EntryState::Open);
    let all_submitted_or_approved = !week_entries.is_empty() && !has_open;

    let submit_status = use_signal(|| None::<String>);

    let current_mode = *view_mode.read();
    let sel_offset = *selected_day_offset.read();
    let is_this_week = ws == iso_week_monday(today);
    let range_label = format!("{} – {}", ws.format("%d %b"), week_end.format("%d %b %Y"));

    rsx! {
        div {
            // Header: title + last-saved + view toggle
            div { class: "ts-header",
                h1 { class: "page-title", "Timesheet" }
                span { class: "ts-saved", "{format_hm(week_total)} this week" }
                Segmented {
                    items: vec!["Day".to_string(), "Week".to_string(), "Calendar".to_string()],
                    active: match current_mode {
                        ViewMode::Day => "Day",
                        ViewMode::Week => "Week",
                        ViewMode::Calendar => "Calendar",
                    }
                        .to_string(),
                    onselect: move |v: String| {
                        view_mode
                            .set(
                                match v.as_str() {
                                    "Day" => ViewMode::Day,
                                    "Calendar" => ViewMode::Calendar,
                                    _ => ViewMode::Week,
                                },
                            )
                    },
                }
            }

            // Toolbar: add entry + week pager
            div { class: "ts-toolbar",
                Link { to: Route::TimeList {}, class: "ts-add", "aria-label": "Add entry", "+" }
                div { class: "ts-pager",
                    button {
                        class: "ts-pager-btn prev",
                        "aria-label": "Previous week",
                        onclick: move |_| week_start.set(ws - Duration::days(7)),
                        "←"
                    }
                    div { class: "ts-pager-label",
                        span { style: "color: var(--color-text-muted);", "▦" }
                        span { class: "cur", if is_this_week { "This week" } else { "Week" } }
                        span { class: "ts-pager-range", "{range_label}" }
                    }
                    button {
                        class: "ts-pager-btn next",
                        "aria-label": "Next week",
                        onclick: move |_| week_start.set(ws + Duration::days(7)),
                        "→"
                    }
                }
                if !is_this_week {
                    button {
                        class: "btn btn-ghost btn-sm",
                        onclick: move |_| {
                            let t = chrono::Utc::now().date_naive();
                            week_start.set(iso_week_monday(t));
                            selected_day_offset.set(t.weekday().num_days_from_monday() as i64);
                        },
                        "Today"
                    }
                }
            }

            // Content
            match &*entries.read() {
                None => rsx! {
                    div { class: "text-muted text-sm", "Loading…" }
                },
                Some(Err(e)) => rsx! {
                    div { class: "alert alert-danger", "{e}" }
                },
                Some(Ok(_)) => match current_mode {
                    ViewMode::Week => rsx! {
                        {render_week_view(&week_entries, &daily_totals, ws, today, &project_names, &task_names)}
                        div { class: "ts-submit-bar",
                            if all_submitted_or_approved {
                                span { class: "badge badge-success", "Submitted" }
                            } else if !week_entries.is_empty() && has_open {
                                div { class: "ts-submit",
                                    button {
                                        class: "ts-submit-main",
                                        disabled: has_non_open,
                                        onclick: move |_| {
                                            let ws_str = ws.to_string();
                                            let mut entries = entries;
                                            let mut submit_status = submit_status;
                                            spawn(async move {
                                                match server_fns::submit_week(ws_str).await {
                                                    Ok(_) => {
                                                        submit_status.set(None);
                                                        entries.restart();
                                                    }
                                                    Err(e) => submit_status.set(Some(format!("{e}"))),
                                                }
                                            });
                                        },
                                        "Submit week for approval"
                                    }
                                    button { class: "ts-submit-caret", "aria-label": "More", "▾" }
                                }
                            }
                            if let Some(err) = &*submit_status.read() {
                                span { style: "color: var(--color-danger); font-size: var(--font-size-sm); margin-left: var(--space-3);",
                                    "{err}"
                                }
                            }
                        }
                    },
                    ViewMode::Day => rsx! {
                        {render_day_view(&by_day, daily_totals.as_slice(), ws, sel_offset, selected_day_offset, &project_names, &task_names)}
                    },
                    ViewMode::Calendar => rsx! {
                        {render_calendar_view(&by_day, &daily_totals, week_total, ws, today, &CalLabels { projects: &project_names, tasks: &task_names, clients: &project_client })}
                    },
                },
            }
        }
    }
}

/// Label lookups shared by the calendar renderer.
struct CalLabels<'a> {
    projects: &'a HashMap<Uuid, String>,
    tasks: &'a HashMap<Uuid, String>,
    /// project_id -> (client name, currency).
    clients: &'a HashMap<Uuid, (String, String)>,
}

/// A calendar event's pre-computed placement and labels.
struct CalEvent {
    top: i32,
    height: i32,
    project: String,
    task: String,
    duration: String,
    client: String,
}

fn render_calendar_view(
    by_day: &[Vec<TimeEntry>; 7],
    daily_totals: &[i32],
    week_total: i32,
    week_start: NaiveDate,
    today: NaiveDate,
    labels: &CalLabels,
) -> Element {
    // Pixels per hour. Entries are placed by *duration* (Harvest's duration mode):
    // stacked from the top of the day, height proportional to minutes.
    const CAL_HOUR: i32 = 48;
    let max_min = daily_totals.iter().copied().max().unwrap_or(0);
    // At least 8 rows so a light week still reads as a calendar.
    let max_hours = ((max_min + 59) / 60).max(8);

    let today_off = {
        let o = (today - week_start).num_days();
        (0..7).contains(&o).then_some(o as usize)
    };
    let col_class = |i: usize| {
        if today_off == Some(i) {
            "ts-cal-col today"
        } else if i >= 5 {
            "ts-cal-col weekend"
        } else {
            "ts-cal-col"
        }
    };
    let head_class = |i: usize| {
        if today_off == Some(i) {
            "ts-cal-dayhead today"
        } else if i >= 5 {
            "ts-cal-dayhead weekend"
        } else {
            "ts-cal-dayhead"
        }
    };

    // Pre-compute each entry's placement, stacked from the top of its day.
    let mut day_events: Vec<Vec<CalEvent>> = Vec::with_capacity(7);
    for day in by_day.iter() {
        let mut cum = 0i32;
        let mut evs = Vec::new();
        for e in day {
            let top = cum * CAL_HOUR / 60;
            let height = (e.minutes * CAL_HOUR / 60).max(20);
            cum += e.minutes;
            let client = labels
                .clients
                .get(&e.project_id)
                .map(|(name, currency)| format!("{name} · {currency}"))
                .unwrap_or_default();
            evs.push(CalEvent {
                top,
                height,
                project: labels
                    .projects
                    .get(&e.project_id)
                    .cloned()
                    .unwrap_or_else(|| "Untitled".into()),
                task: labels.tasks.get(&e.task_id).cloned().unwrap_or_default(),
                duration: format_hm(e.minutes),
                client,
            });
        }
        day_events.push(evs);
    }

    rsx! {
        div { class: "ts-cal",
            div { class: "ts-cal-scroll",
                div { class: "ts-cal-head",
                    span {}
                    for i in 0..7 {
                        {
                            let d = week_start + Duration::days(i as i64);
                            rsx! {
                                div { class: "{head_class(i)}",
                                    div { class: "ts-cal-dayname", "{DAY_LABELS[i]} {d.day()}" }
                                    div { class: "ts-cal-daytotal", "{format_hm(daily_totals[i])}" }
                                }
                            }
                        }
                    }
                    div { class: "ts-cal-weektot",
                        div { class: "ts-cal-weektot-label", "Week total" }
                        div { class: "ts-cal-weektot-value", "{format_hm(week_total)}" }
                    }
                }

                div { class: "ts-cal-grid",
                    div { class: "ts-cal-rail",
                        for h in 0..max_hours {
                            div { class: "ts-cal-hour",
                                span { class: "ts-cal-hour-label", "{h + 1}hr" }
                            }
                        }
                    }
                    for i in 0..7 {
                        div { class: "{col_class(i)}",
                            for ev in day_events[i].iter() {
                                div {
                                    class: "ts-cal-event",
                                    style: "top: {ev.top}px; height: {ev.height}px;",
                                    div { class: "ts-cal-ev-project",
                                        span { class: "ts-cal-ev-name", "{ev.project}" }
                                        span { class: "ts-cal-ev-dur", "{ev.duration}" }
                                    }
                                    if !ev.task.is_empty() {
                                        div { class: "ts-cal-ev-task", "{ev.task}" }
                                    }
                                    if !ev.client.is_empty() {
                                        div { class: "ts-cal-ev-client", "{ev.client}" }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "ts-cal-tail" }
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
    today: NaiveDate,
    project_names: &HashMap<Uuid, String>,
    task_names: &HashMap<Uuid, String>,
) -> Element {
    // Group by (project_id, task_id) into per-day minutes, preserving order.
    let mut row_keys: Vec<(Uuid, Uuid)> = Vec::new();
    let mut row_map: HashMap<(Uuid, Uuid), [i32; 7]> = HashMap::new();
    for entry in entries {
        let offset = (entry.spent_date - week_start).num_days();
        if !(0..7).contains(&offset) {
            continue;
        }
        let row = row_map
            .entry((entry.project_id, entry.task_id))
            .or_insert_with(|| {
                row_keys.push((entry.project_id, entry.task_id));
                [0i32; 7]
            });
        row[offset as usize] += entry.minutes;
    }

    let today_off = {
        let o = (today - week_start).num_days();
        (0..7).contains(&o).then_some(o as usize)
    };
    let day_class = |i: usize, base: &str| {
        if today_off == Some(i) {
            format!("{base} today")
        } else if i >= 5 {
            format!("{base} weekend")
        } else {
            base.to_string()
        }
    };

    rsx! {
        div { class: "ts-grid-card",
            div { class: "ts-grid-scroll",
                // Header row
                div { class: "ts-row ts-head",
                    span {}
                    for i in 0..7 {
                        {
                            let d = week_start + Duration::days(i as i64);
                            rsx! {
                                span { class: "{day_class(i, \"ts-daycol\")}",
                                    span { class: "ts-dayname", "{DAY_LABELS[i]}" }
                                    span { class: "ts-daynum", "{d.format(\"%d %b\")}" }
                                }
                            }
                        }
                    }
                    span { class: "ts-total-head", "Total" }
                    span {}
                }

                if row_keys.is_empty() {
                    div { class: "empty-state",
                        div { class: "empty-state-icon", "🗓" }
                        div { class: "empty-state-title", "No time this week" }
                        p { class: "text-muted text-sm", "Add an entry to start filling your timesheet." }
                    }
                }

                // Project rows
                for key in row_keys.iter() {
                    {
                        let (pid, tid) = *key;
                        let proj = project_names.get(&pid).cloned().unwrap_or_else(|| pid.to_string());
                        let task = task_names.get(&tid).cloned().unwrap_or_else(|| "\u{2014}".into());
                        let row = row_map.get(key).copied().unwrap_or([0; 7]);
                        let row_total: i32 = row.iter().sum();
                        rsx! {
                            div { class: "ts-row ts-body",
                                div { class: "ts-project",
                                    button { class: "ts-project-icon", "aria-label": "Task", "▤" }
                                    div {
                                        div { class: "ts-project-title", strong { "{proj}" } }
                                        div { class: "ts-project-task", "{task}" }
                                    }
                                }
                                for i in 0..7 {
                                    {
                                        let mins = row[i];
                                        let cls = if mins == 0 {
                                            "ts-cell-box empty".to_string()
                                        } else if today_off == Some(i) {
                                            "ts-cell-box today".to_string()
                                        } else {
                                            "ts-cell-box".to_string()
                                        };
                                        rsx! {
                                            div { class: "ts-cell",
                                                div { class: "{cls}",
                                                    if mins > 0 {
                                                        "{format_hm(mins)}"
                                                    } else {
                                                        "\u{2013}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "ts-rowtotal", "{format_hm(row_total)}" }
                                div { style: "text-align: center;",
                                    button { class: "ts-del", "aria-label": "Remove row", "\u{00d7}" }
                                }
                            }
                        }
                    }
                }

                // Footer: add-row + column totals
                div { class: "ts-row ts-foot",
                    div {
                        Link { to: Route::TimeList {}, class: "ts-addrow",
                            span { class: "plus", "\u{ff0b}" }
                            "Add row"
                        }
                    }
                    for i in 0..7 {
                        {
                            let t = daily_totals[i];
                            let cls = if t == 0 {
                                "ts-coltotal empty".to_string()
                            } else if today_off == Some(i) {
                                "ts-coltotal today".to_string()
                            } else {
                                "ts-coltotal".to_string()
                            };
                            rsx! {
                                div { class: "{cls}",
                                    if t > 0 {
                                        "{format_hm(t)}"
                                    } else {
                                        "0"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "ts-grandtotal", "{format_hm(daily_totals.iter().sum::<i32>())}" }
                    div {}
                }
            }
        }
    }
}
