use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate};
use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::controls::Segmented;
use crate::models::time_entry::TimeEntry;
use crate::route::Route;
use crate::server_fns;

/// `H:MM` clock format from integer minutes (the design's cell/total format).
/// Delegates to the core formatter so duration display has one source of truth.
fn format_hm(total_minutes: i32) -> String {
    horae_core::duration::format_hhmm(total_minutes.max(0) as u32)
}

/// Offset (0 = Mon .. 6 = Sun) of `today` within the week starting `week_start`,
/// or `None` when today falls outside that week.
fn today_offset(today: NaiveDate, week_start: NaiveDate) -> Option<usize> {
    let o = (today - week_start).num_days();
    (0..7).contains(&o).then_some(o as usize)
}

/// A weekday column's CSS class: `base`, plus a `today`/`weekend` modifier.
fn day_col_class(base: &str, today_off: Option<usize>, i: usize) -> String {
    if today_off == Some(i) {
        format!("{base} today")
    } else if i >= 5 {
        format!("{base} weekend")
    } else {
        base.to_string()
    }
}

/// A week-grid value cell's class: `base`, plus `empty` when zero or `today`
/// when it's today's column.
fn value_cell_class(base: &str, minutes: i32, today_off: Option<usize>, i: usize) -> String {
    if minutes == 0 {
        format!("{base} empty")
    } else if today_off == Some(i) {
        format!("{base} today")
    } else {
        base.to_string()
    }
}

/// Map a list-returning resource's loaded value, or yield `R::default()` while it
/// is still loading or errored — collapses the repeated
/// `read().as_ref().and_then(...).map(...).unwrap_or_default()` boilerplate.
fn from_list<T: 'static, E: 'static, R: Default>(
    res: &Resource<Result<Vec<T>, E>>,
    f: impl FnOnce(&[T]) -> R,
) -> R {
    res.read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|v| f(v))
        .unwrap_or_default()
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

    // Lookups and grid data are memoized so they rebuild only when their
    // resources (or the selected week) change — not on every render, e.g. each
    // keystroke in the add-entry modal.
    let project_names = use_memo(move || -> HashMap<Uuid, String> {
        from_list(&projects, |ps| {
            ps.iter().map(|p| (p.id, p.name.clone())).collect()
        })
    });
    let task_names = use_memo(move || -> HashMap<Uuid, String> {
        from_list(&tasks, |ts| {
            ts.iter().map(|t| (t.id, t.name.clone())).collect()
        })
    });
    // project_id -> (client name, project currency), for the calendar event's
    // "Client · CUR" line.
    let project_client = use_memo(move || -> HashMap<Uuid, (String, String)> {
        let client_names: HashMap<Uuid, String> = from_list(&clients, |cs| {
            cs.iter().map(|c| (c.id, c.name.clone())).collect()
        });
        from_list(&projects, |ps| {
            ps.iter()
                .map(|p| {
                    let name = client_names.get(&p.client_id).cloned().unwrap_or_default();
                    (p.id, (name, p.currency.clone()))
                })
                .collect()
        })
    });

    let ws = *week_start.read();
    let week_end = ws + Duration::days(6);

    // Entries for the visible week, grouped by weekday, with per-day totals.
    let week_entries = use_memo(move || -> Vec<TimeEntry> {
        let ws = week_start();
        let we = ws + Duration::days(6);
        from_list(&entries, |es| {
            es.iter()
                .filter(|e| e.spent_date >= ws && e.spent_date <= we)
                .cloned()
                .collect()
        })
    });
    let by_day = use_memo(move || -> [Vec<TimeEntry>; 7] {
        let ws = week_start();
        let mut by_day: [Vec<TimeEntry>; 7] = Default::default();
        for entry in week_entries.read().iter() {
            let offset = (entry.spent_date - ws).num_days();
            if (0..7).contains(&offset) {
                by_day[offset as usize].push(entry.clone());
            }
        }
        by_day
    });
    let daily_totals = use_memo(move || -> Vec<i32> {
        by_day
            .read()
            .iter()
            .map(|d| d.iter().map(|e| e.minutes).sum())
            .collect()
    });
    let week_total: i32 = daily_totals.read().iter().sum();

    // Submission state of the week's entries (Open = still editable).
    let has_non_open = week_entries
        .read()
        .iter()
        .any(|e| e.state != horae_core::types::EntryState::Open);
    let has_open = week_entries
        .read()
        .iter()
        .any(|e| e.state == horae_core::types::EntryState::Open);
    let all_submitted_or_approved = !week_entries.read().is_empty() && !has_open;

    let submit_status = use_signal(|| None::<String>);

    // Add–entry modal state. `add_open` holds the date the new entry is for
    // (None = closed); the rest back the form fields.
    let mut add_open = use_signal(|| None::<NaiveDate>);
    let mut add_project = use_signal(String::new);
    let mut add_task = use_signal(String::new);
    let mut add_notes = use_signal(String::new);
    let mut add_duration = use_signal(|| "0:00".to_string());
    let mut add_error = use_signal(|| None::<String>);
    let mut add_saving = use_signal(|| false);

    // Open the modal for `date`, defaulting the selects to the first project/task.
    let open_add = use_callback(move |date: NaiveDate| {
        let first_project = from_list(&projects, |ps| {
            ps.first().map(|p| p.id.to_string()).unwrap_or_default()
        });
        let first_task = from_list(&tasks, |ts| {
            ts.first().map(|t| t.id.to_string()).unwrap_or_default()
        });
        add_project.set(first_project);
        add_task.set(first_task);
        add_notes.set(String::new());
        add_duration.set("0:00".to_string());
        add_error.set(None);
        add_open.set(Some(date));
    });

    // Options for the modal selects: (id, label).
    let project_options = use_memo(move || -> Vec<(String, String)> {
        from_list(&projects, |ps| {
            ps.iter()
                .map(|p| {
                    let label = match &p.code {
                        Some(code) => format!("[{code}] {}", p.name),
                        None => p.name.clone(),
                    };
                    (p.id.to_string(), label)
                })
                .collect()
        })
    });
    let task_options = use_memo(move || -> Vec<(String, String)> {
        from_list(&tasks, |ts| {
            ts.iter()
                .map(|t| (t.id.to_string(), t.name.clone()))
                .collect()
        })
    });

    // The "+" button adds for today when it's in the viewed week, else Monday.
    let add_default_date = if (0..7).contains(&(today - ws).num_days()) {
        today
    } else {
        ws
    };

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
                button {
                    class: "ts-add",
                    "aria-label": "Add entry",
                    onclick: move |_| open_add.call(add_default_date),
                    "+"
                }
                div { class: "ts-pager",
                    button {
                        class: "ts-pager-btn prev",
                        "aria-label": "Previous week",
                        onclick: move |_| week_start.set(ws - Duration::days(7)),
                        "←"
                    }
                    div { class: "ts-pager-label",
                        span { class: "text-faint", "▦" }
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
                        {render_week_view(&week_entries.read(), &daily_totals.read(), ws, today, &project_names.read(), &task_names.read())}
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
                                span { class: "text-danger text-sm ml-3",
                                    "{err}"
                                }
                            }
                        }
                    },
                    ViewMode::Day => rsx! {
                        {render_day_view(&by_day.read(), &daily_totals.read(), ws, sel_offset, selected_day_offset, &project_names.read(), &task_names.read())}
                    },
                    ViewMode::Calendar => rsx! {
                        {render_calendar_view(&by_day.read(), &daily_totals.read(), week_total, ws, today, &CalLabels { projects: &project_names.read(), tasks: &task_names.read(), clients: &project_client.read() }, open_add)}
                    },
                },
            }

            // Add–entry modal (opened by "+" or by clicking a calendar day).
            if let Some(date) = *add_open.read() {
                div {
                    class: "ts-modal-overlay",
                    onclick: move |_| add_open.set(None),
                    div {
                        class: "ts-modal",
                        onclick: move |e| e.stop_propagation(),
                        div { class: "ts-modal-title", "New time entry for {date.format(\"%A, %-d %b\")}" }
                        div { class: "ts-modal-body",
                            label { class: "form-label", "Project / Task" }
                            select {
                                class: "form-select",
                                value: "{add_project}",
                                onchange: move |e| add_project.set(e.value()),
                                for (id , label) in project_options.read().iter() {
                                    option { value: "{id}", "{label}" }
                                }
                            }
                            select {
                                class: "form-select",
                                value: "{add_task}",
                                onchange: move |e| add_task.set(e.value()),
                                for (id , label) in task_options.read().iter() {
                                    option { value: "{id}", "{label}" }
                                }
                            }
                            div { class: "ts-modal-row",
                                input {
                                    class: "form-input ts-modal-notes",
                                    placeholder: "Notes (optional)",
                                    value: "{add_notes}",
                                    oninput: move |e| add_notes.set(e.value()),
                                }
                                input {
                                    class: "form-input ts-modal-duration",
                                    "aria-label": "Duration",
                                    value: "{add_duration}",
                                    oninput: move |e| add_duration.set(e.value()),
                                }
                            }
                            if let Some(err) = &*add_error.read() {
                                div { class: "ts-modal-error", "{err}" }
                            }
                            div { class: "ts-modal-actions",
                                button {
                                    class: "btn btn-primary",
                                    disabled: add_saving(),
                                    onclick: move |_| {
                                        let project_id = add_project.read().clone();
                                        let task_id = add_task.read().clone();
                                        if project_id.is_empty() || task_id.is_empty() {
                                            add_error.set(Some("Select a project and task.".to_string()));
                                            return;
                                        }
                                        // Parse cap keeps the u32 -> i32 cast lossless: a day
                                        // can't hold more than 24h, and 0 is not an entry.
                                        const MAX_ENTRY_MINUTES: u32 = 24 * 60;
                                        let minutes = match horae_core::duration::parse(&add_duration.read()) {
                                            Ok(0) => {
                                                add_error
                                                    .set(Some("Duration must be greater than zero.".to_string()));
                                                return;
                                            }
                                            Ok(m) if m > MAX_ENTRY_MINUTES => {
                                                add_error
                                                    .set(Some("Duration can't exceed 24 hours.".to_string()));
                                                return;
                                            }
                                            Ok(m) => m as i32,
                                            Err(_) => {
                                                add_error
                                                    .set(Some("Enter a duration like 1:30.".to_string()));
                                                return;
                                            }
                                        };
                                        let notes = {
                                            let n = add_notes.read().trim().to_string();
                                            (!n.is_empty()).then_some(n)
                                        };
                                        let spent = date.to_string();
                                        let mut entries = entries;
                                        add_saving.set(true);
                                        add_error.set(None);
                                        spawn(async move {
                                            match server_fns::create_time_entry(
                                                    project_id,
                                                    task_id,
                                                    spent,
                                                    minutes,
                                                    notes,
                                                    true,
                                                )
                                                .await
                                            {
                                                Ok(_) => {
                                                    add_open.set(None);
                                                    entries.restart();
                                                }
                                                Err(e) => add_error.set(Some(format!("Could not save: {e}"))),
                                            }
                                            add_saving.set(false);
                                        });
                                    },
                                    if add_saving() { "Saving…" } else { "Save entry" }
                                }
                                button {
                                    class: "btn btn-ghost",
                                    onclick: move |_| add_open.set(None),
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
    open_add: Callback<NaiveDate>,
) -> Element {
    // Pixels per hour. Entries are placed by *duration* (Harvest's duration mode):
    // stacked from the top of the day, height proportional to minutes.
    const CAL_HOUR: i32 = 48;
    let max_min = daily_totals.iter().copied().max().unwrap_or(0);
    // At least 8 rows so a light week still reads as a calendar.
    let max_hours = ((max_min + 59) / 60).max(8);

    let today_off = today_offset(today, week_start);
    let col_class = |i: usize| day_col_class("ts-cal-col", today_off, i);
    let head_class = |i: usize| day_col_class("ts-cal-dayhead", today_off, i);

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
                        div {
                            class: "{col_class(i)}",
                            onclick: move |_| open_add.call(week_start + Duration::days(i as i64)),
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
        div { class: "flex gap-1 mb-4",
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
            h3 { class: "mb-4 text-default",
                "{day_date.format(\"%A, %B %d, %Y\")}"
            }

            if day_entries.is_empty() {
                div { class: "text-muted text-sm p-8 text-center",
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

            div { class: "mt-4 text-right p-2",
                span { class: "text-muted text-sm", "Day total: " }
                span { class: "text-mono font-semibold text-primary",
                    "{format_hm(total)}"
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

    let today_off = today_offset(today, week_start);
    let day_class = |i: usize, base: &str| day_col_class(base, today_off, i);

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
                                        let cls = value_cell_class("ts-cell-box", mins, today_off, i);
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
                                div { class: "text-center",
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
                            let cls = value_cell_class("ts-coltotal", t, today_off, i);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn format_hm_pads_minutes() {
        assert_eq!(format_hm(65), "1:05");
    }

    #[test]
    fn format_hm_clamps_negative_to_zero() {
        assert_eq!(format_hm(-5), "0:00");
    }

    #[test]
    fn today_offset_is_zero_on_the_monday() {
        let monday = ymd(2026, 7, 13);
        assert_eq!(today_offset(monday, monday), Some(0));
    }

    #[test]
    fn today_offset_is_six_on_the_sunday() {
        let monday = ymd(2026, 7, 13);
        assert_eq!(today_offset(ymd(2026, 7, 19), monday), Some(6));
    }

    #[test]
    fn today_offset_is_none_before_the_week() {
        let monday = ymd(2026, 7, 13);
        assert_eq!(today_offset(ymd(2026, 7, 12), monday), None);
    }

    #[test]
    fn today_offset_is_none_after_the_week() {
        let monday = ymd(2026, 7, 13);
        assert_eq!(today_offset(ymd(2026, 7, 20), monday), None);
    }

    #[test]
    fn day_col_class_marks_today() {
        assert_eq!(day_col_class("c", Some(2), 2), "c today");
    }

    #[test]
    fn day_col_class_marks_weekend() {
        assert_eq!(day_col_class("c", None, 5), "c weekend");
    }

    #[test]
    fn day_col_class_today_wins_over_weekend() {
        assert_eq!(day_col_class("c", Some(6), 6), "c today");
    }

    #[test]
    fn day_col_class_plain_weekday() {
        assert_eq!(day_col_class("c", None, 1), "c");
    }

    #[test]
    fn value_cell_class_empty_wins_over_today() {
        assert_eq!(value_cell_class("v", 0, Some(2), 2), "v empty");
    }

    #[test]
    fn value_cell_class_marks_today_when_nonzero() {
        assert_eq!(value_cell_class("v", 30, Some(2), 2), "v today");
    }
}
