use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate};
use dioxus::html::geometry::PixelsVector2D;
use dioxus::prelude::*;
use tracing::error;
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

/// Create, update, or (when `minutes` is 0) delete a time entry — `existing` is
/// the entry to change, or `None` to create one. Shared by the week grid cells
/// and the entry dialog so both save the same way.
#[expect(
    clippy::too_many_arguments,
    reason = "one shared create/update/delete dispatch mirroring the entry's fields"
)]
async fn persist_entry(
    existing: Option<Uuid>,
    project_id: String,
    task_id: String,
    day: NaiveDate,
    minutes: i32,
    notes: Option<String>,
    billable: bool,
    start_minute: Option<i32>,
) -> Result<(), ServerFnError> {
    match (existing, minutes) {
        (Some(id), 0) => server_fns::delete_time_entry(id.to_string())
            .await
            .map(|_| ()),
        (Some(id), m) => {
            server_fns::update_time_entry(id.to_string(), m, notes, billable, start_minute)
                .await
                .map(|_| ())
        }
        (None, m) if m > 0 => server_fns::create_time_entry(
            project_id,
            task_id,
            day.to_string(),
            m,
            notes,
            billable,
            start_minute,
        )
        .await
        .map(|_| ()),
        _ => Ok(()),
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum ViewMode {
    Day,
    #[default]
    Week,
    Calendar,
}

impl std::fmt::Display for ViewMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ViewMode::Day => "day",
            ViewMode::Week => "week",
            ViewMode::Calendar => "calendar",
        })
    }
}

impl std::str::FromStr for ViewMode {
    type Err = std::convert::Infallible;
    // Unknown values fall back to Week so a stray URL never fails to route.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "day" => ViewMode::Day,
            "calendar" => ViewMode::Calendar,
            _ => ViewMode::Week,
        })
    }
}

/// The timesheet's anchor day, carried in the URL path
/// (`/timesheet/<view>/YYYY-MM-DD`). The week shown is the ISO week containing
/// it; in Day view it is the selected day.
#[derive(Clone, Copy, PartialEq)]
pub struct Anchor(pub NaiveDate);

impl Default for Anchor {
    fn default() -> Self {
        Anchor(chrono::Utc::now().date_naive())
    }
}

impl std::fmt::Display for Anchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d"))
    }
}

impl std::str::FromStr for Anchor {
    type Err = std::convert::Infallible;
    // A malformed date falls back to today rather than failing the route.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Anchor)
            .unwrap_or_default())
    }
}

/// Return the Monday of the ISO week containing `date`.
fn iso_week_monday(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

const DAY_LABELS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// How many days the Calendar view shows at once.
#[derive(Clone, Copy, PartialEq)]
enum CalSpan {
    /// Mon–Sun.
    Week,
    /// Mon–Fri.
    WorkWeek,
    /// Just the anchor day.
    Day,
}

impl CalSpan {
    /// Weekday indices (0 = Mon) to render, given the anchor day's own index.
    fn visible_days(self, anchor: usize) -> Vec<usize> {
        match self {
            CalSpan::Week => (0..7).collect(),
            CalSpan::WorkWeek => (0..5).collect(),
            CalSpan::Day => vec![anchor.min(6)],
        }
    }

    /// Selector label. Distinct from the Day/Week view toggle so the two
    /// segmented controls don't read as duplicates.
    fn label(self) -> &'static str {
        match self {
            CalSpan::Week => "7 days",
            CalSpan::WorkWeek => "5 days",
            CalSpan::Day => "1 day",
        }
    }
}

#[component]
pub fn Timesheet(view: ViewMode, date: Anchor) -> Element {
    let today = chrono::Utc::now().date_naive();
    // View, week and selected day all derive from the URL path
    // (/timesheet/<view>/<date>), so switching views or navigating is shareable
    // and works with the browser's back/forward. Actions push a new route.
    let view_mode = use_memo(use_reactive!(|(view,)| view));
    let week_start = use_memo(use_reactive!(|(date,)| iso_week_monday(date.0)));
    // Which day is selected within the week (0 = Monday .. 6 = Sunday) for Day view.
    let selected_day_offset =
        use_memo(use_reactive!(
            |(date,)| date.0.weekday().num_days_from_monday() as i64
        ));

    // Push a new view/anchor to the URL.
    let go = use_callback(move |(v, anchor): (ViewMode, NaiveDate)| {
        navigator().push(Route::Timesheet {
            view: v,
            date: Anchor(anchor),
        });
    });
    // Selecting a day in the Day-view strip navigates to that day.
    let select_day = use_callback(move |i: i64| {
        go.call((ViewMode::Day, week_start() + Duration::days(i)));
    });

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
    // When set, the modal edits this existing entry instead of creating one; its
    // billable flag is carried through (the modal doesn't expose it).
    let mut editing = use_signal(|| None::<Uuid>);
    let mut edit_billable = use_signal(|| true);
    // The entry's optional start time (minutes since midnight); None = untimed.
    // Set by a calendar drag or when editing a timed entry; carried into save.
    let mut add_start = use_signal(|| None::<i32>);

    // Whether the entry modal's primary action starts a timer (Harvest-style):
    // a new entry on today's column with no duration typed yet. Once a duration
    // or a start time is set the entry is clearly a fixed one, so the primary
    // saves instead of starting a timer.
    let timer_mode = use_memo(move || {
        let Some(date) = *add_open.read() else {
            return false;
        };
        editing.read().is_none()
            && date == today
            && add_start.read().is_none()
            && !matches!(horae_core::duration::parse(&add_duration.read()), Ok(m) if m > 0)
    });

    // Open the modal to create a new entry for `date`, defaulting the selects to
    // the first project/task.
    let open_add = use_callback(move |date: NaiveDate| {
        let first_project = from_list(&projects, |ps| {
            ps.first().map(|p| p.id.to_string()).unwrap_or_default()
        });
        let first_task = from_list(&tasks, |ts| {
            ts.first().map(|t| t.id.to_string()).unwrap_or_default()
        });
        editing.set(None);
        add_project.set(first_project);
        add_task.set(first_task);
        add_notes.set(String::new());
        add_duration.set("0:00".to_string());
        add_start.set(None);
        add_error.set(None);
        add_open.set(Some(date));
    });

    // Open the modal to edit an existing entry, pre-filled from it. Project and
    // task are read-only in edit mode (the update only changes duration/notes).
    let open_edit = use_callback(move |e: TimeEntry| {
        editing.set(Some(e.id));
        add_project.set(e.project_id.to_string());
        add_task.set(e.task_id.to_string());
        add_notes.set(e.notes.clone().unwrap_or_default());
        add_duration.set(format_hm(e.minutes));
        add_start.set(e.start_minute);
        edit_billable.set(e.billable);
        add_error.set(None);
        add_open.set(Some(e.spent_date));
    });

    // How many days the Calendar shows (week / work-week / single day).
    let mut cal_span = use_signal(|| CalSpan::Week);

    // Calendar drag: the slot/entry being manipulated, committed on release.
    let cal_drag = use_signal(|| None::<CalDrag>);

    // The free calendar slot the cursor is over: (column, snapped minute). Drives
    // the cursor-following "+ Add time" hint.
    let add_hint = use_signal(|| None::<(usize, i32)>);
    let drag_commit = use_callback(move |d: CalDrag| {
        let ws = *week_start.read();
        let clamp = |start: i32, dur: i32| {
            horae_core::time_of_day::clamp_to_day(start.clamp(0, 1439) as u16, dur.max(0) as u32)
                as i32
        };
        // Run a mutation and refresh the week's entries when it succeeds. Shared
        // by every drag that writes (move, resize, reorder).
        let commit = move |fut: std::pin::Pin<Box<dyn std::future::Future<Output = bool>>>| {
            let mut entries = entries;
            spawn(async move {
                if fut.await {
                    entries.restart();
                }
            });
        };
        match d.kind {
            // Draw a new slot → open the entry form prefilled at that hour. A drag
            // sets the duration to the dragged span; a plain click seeds a default
            // one-hour block starting at the clicked time.
            DragKind::Create => {
                let day = ws + Duration::days(d.day as i64);
                let start = d.start_min.min(d.cur_min).clamp(0, 1439);
                let raw = (d.cur_min - d.start_min).abs();
                let dur = if raw >= i32::from(horae_core::time_of_day::MIN_DURATION) {
                    raw
                } else {
                    60
                };
                open_add.call(day);
                add_start.set(Some(start));
                add_duration.set(format_hm(clamp(start, dur)));
            }
            // Move an entry → new start follows the pointer (keeping the grab
            // offset), possibly to another day. No movement → open it for editing.
            DragKind::Move => {
                let Some(entry) = d.entry.clone() else {
                    return;
                };
                let new_start = d.move_start();
                if new_start == d.start_min && d.day == d.orig_day {
                    open_edit.call(entry);
                    return;
                }
                let dur = clamp(new_start, d.orig_dur);
                let date = (ws + Duration::days(d.day as i64)).to_string();
                let id = entry.id.to_string();
                commit(Box::pin(async move {
                    server_fns::reschedule_time_entry(id, date, new_start, dur)
                        .await
                        .is_ok()
                }));
            }
            // Resize an entry → new duration from its start to the pointer.
            DragKind::Resize => {
                let Some(entry) = d.entry.clone() else {
                    return;
                };
                let dur = clamp(d.start_min, d.resize_end() - d.start_min);
                let date = (ws + Duration::days(d.orig_day as i64)).to_string();
                let id = entry.id.to_string();
                let start = d.start_min;
                commit(Box::pin(async move {
                    server_fns::reschedule_time_entry(id, date, start, dur)
                        .await
                        .is_ok()
                }));
            }
            // Reorder an untimed entry within its day's stack. No move (or a drop
            // on another day) → treat as a click and open it for editing.
            DragKind::Reorder => {
                let Some(entry) = d.entry.clone() else {
                    return;
                };
                // Drop column = target day; the same call reorders within a day and
                // moves the entry to another day (its spent_date follows).
                let target_date = ws + Duration::days(d.day as i64);
                let mut ordered = entries
                    .read()
                    .as_ref()
                    .and_then(|r| r.as_ref().ok())
                    .map(|all| {
                        let day: Vec<TimeEntry> = all
                            .iter()
                            .filter(|e| e.spent_date == target_date)
                            .cloned()
                            .collect();
                        untimed_ordered(&day)
                    })
                    .unwrap_or_default();
                let before: Vec<Uuid> = ordered.iter().map(|e| e.id).collect();
                // Drop the moved entry from the target list (present only on a
                // same-day reorder) and re-insert it at the drop position.
                ordered.retain(|e| e.id != entry.id);
                let mut cum = 0i32;
                let mut to = ordered.len();
                for (idx, e) in ordered.iter().enumerate() {
                    if d.cur_min < cum + e.minutes / 2 {
                        to = idx;
                        break;
                    }
                    cum += e.minutes;
                }
                ordered.insert(to, entry.clone());
                let after: Vec<Uuid> = ordered.iter().map(|e| e.id).collect();
                // Same day and unchanged order → treat as a click and open editing.
                if d.day == d.orig_day && after == before {
                    open_edit.call(entry);
                    return;
                }
                let ids: Vec<String> = after.iter().map(|id| id.to_string()).collect();
                let date = target_date.to_string();
                commit(Box::pin(async move {
                    server_fns::reorder_untimed_entries(date, ids).await.is_ok()
                }));
            }
        }
    });

    // Start a timer for an existing entry's project/task (the Day-view "Start"
    // action, Harvest-style resume).
    let start_entry = use_callback(move |e: TimeEntry| {
        let mut entries = entries;
        spawn(async move {
            match server_fns::start_timer(
                e.project_id.to_string(),
                e.task_id.to_string(),
                e.notes.clone(),
            )
            .await
            {
                Ok(_) => entries.restart(),
                Err(err) => error!("Start timer error: {err}"),
            }
        });
    });

    // ── Editable week grid ───────────────────────────────────────────────────
    // Rows added via "Add row" that have no entries yet this week.
    let mut pending_rows = use_signal(Vec::<(Uuid, Uuid)>::new);
    let mut addrow_open = use_signal(|| false);
    let mut addrow_project = use_signal(String::new);
    let mut addrow_task = use_signal(String::new);

    // Commit a grid cell: create, update, or clear the entry behind it, then
    // reload. Notes/billable of an updated entry are preserved.
    let commit_cell = use_callback(move |edit: CellEdit| {
        let (notes, billable) = edit
            .existing
            .and_then(|id| {
                week_entries
                    .read()
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| (e.notes.clone(), e.billable))
            })
            .unwrap_or((None, true));
        let mut entries = entries;
        spawn(async move {
            let res = persist_entry(
                edit.existing,
                edit.project_id.to_string(),
                edit.task_id.to_string(),
                edit.day,
                edit.minutes,
                notes,
                billable,
                None, // Week-grid cells are untimed (no time of day)
            )
            .await;
            if res.is_ok() {
                entries.restart();
            }
        });
    });

    // Remove a row: drop a pending one, or delete every entry it holds this week.
    let remove_row = use_callback(move |key: (Uuid, Uuid)| {
        pending_rows.write().retain(|k| *k != key);
        let ids: Vec<Uuid> = week_entries
            .read()
            .iter()
            .filter(|e| e.project_id == key.0 && e.task_id == key.1)
            .map(|e| e.id)
            .collect();
        if ids.is_empty() {
            return;
        }
        let mut entries = entries;
        spawn(async move {
            for id in ids {
                let _ = server_fns::delete_time_entry(id.to_string()).await;
            }
            entries.restart();
        });
    });

    let open_add_row = use_callback(move |()| {
        addrow_project.set(from_list(&projects, |ps| {
            ps.first().map(|p| p.id.to_string()).unwrap_or_default()
        }));
        addrow_task.set(from_list(&tasks, |ts| {
            ts.first().map(|t| t.id.to_string()).unwrap_or_default()
        }));
        addrow_open.set(true);
    });

    let week_actions = WeekActions {
        commit: commit_cell,
        remove_row,
        add_row: open_add_row,
    };

    // Shared validation for the entry dialog's Start-timer / Save actions: a
    // project and task must be picked. Returns the values (notes trimmed) or sets
    // the modal error and yields None.
    let read_pt_notes = use_callback(move |()| -> Option<(String, String, Option<String>)> {
        let project_id = add_project.read().clone();
        let task_id = add_task.read().clone();
        if project_id.is_empty() || task_id.is_empty() {
            add_error.set(Some("Select a project and task.".to_string()));
            return None;
        }
        let notes = {
            let n = add_notes.read().trim().to_string();
            (!n.is_empty()).then_some(n)
        };
        Some((project_id, task_id, notes))
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

    // Pager stepping: Day view moves one day, Week/Calendar a whole week. Moving
    // the anchor date across the week edge rolls the week automatically.
    let step = use_callback(move |forward: bool| {
        let mode = *view_mode.read();
        let days = if mode == ViewMode::Day { 1 } else { 7 };
        let delta = Duration::days(if forward { days } else { -days });
        go.call((mode, date.0 + delta));
    });
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
                        let v = match v.as_str() {
                            "Day" => ViewMode::Day,
                            "Calendar" => ViewMode::Calendar,
                            _ => ViewMode::Week,
                        };
                        go.call((v, date.0));
                    },
                }
                // Calendar-only: how many days the grid spans.
                if current_mode == ViewMode::Calendar {
                    Segmented {
                        items: vec!["7 days".to_string(), "5 days".to_string(), "1 day".to_string()],
                        active: cal_span.read().label().to_string(),
                        onselect: move |v: String| {
                            cal_span
                                .set(match v.as_str() {
                                    "1 day" => CalSpan::Day,
                                    "5 days" => CalSpan::WorkWeek,
                                    _ => CalSpan::Week,
                                });
                        },
                    }
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
                        "aria-label": if current_mode == ViewMode::Day { "Previous day" } else { "Previous week" },
                        onclick: move |_| step.call(false),
                        "←"
                    }
                    div { class: "ts-pager-label",
                        span { class: "text-faint", "▦" }
                        if current_mode == ViewMode::Day {
                            {
                                let d = ws + Duration::days((*selected_day_offset.read()).clamp(0, 6));
                                rsx! {
                                    span { class: "cur", if d == today { "Today" } else { "{d.format(\"%A\")}" } }
                                    span { class: "ts-pager-range", "{d.format(\"%d %b %Y\")}" }
                                }
                            }
                        } else {
                            span { class: "cur", if is_this_week { "This week" } else { "Week" } }
                            span { class: "ts-pager-range", "{range_label}" }
                        }
                    }
                    button {
                        class: "ts-pager-btn next",
                        "aria-label": if current_mode == ViewMode::Day { "Next day" } else { "Next week" },
                        onclick: move |_| step.call(true),
                        "→"
                    }
                }
                if !is_this_week {
                    button {
                        class: "btn btn-ghost btn-sm",
                        onclick: move |_| go.call((current_mode, today)),
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
                        {render_week_view(&week_entries.read(), &daily_totals.read(), ws, today, &project_names.read(), &task_names.read(), &pending_rows.read(), week_actions)}
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
                        {render_day_view(&by_day.read(), &daily_totals.read(), sel_offset, select_day, &project_names.read(), &task_names.read(), open_edit, start_entry)}
                    },
                    ViewMode::Calendar => rsx! {
                        {
                            let visible = cal_span.read().visible_days(*selected_day_offset.read() as usize);
                            render_calendar_view(&by_day.read(), &daily_totals.read(), &visible, ws, today, &CalLabels { projects: &project_names.read(), tasks: &task_names.read(), clients: &project_client.read() }, cal_drag, add_hint, drag_commit)
                        }
                    },
                },
            }

            // Add–entry modal (opened by "+" or by clicking a calendar day).
            if let Some(date) = *add_open.read() {
                div {
                    class: "modal-overlay",
                    onclick: move |_| add_open.set(None),
                    div {
                        class: "modal modal-lg",
                        onclick: move |e| e.stop_propagation(),
                        div { class: "ts-modal-title",
                            if editing.read().is_some() { "Edit time entry" } else { "New time entry" }
                            " for {date.format(\"%A, %d %b\")}"
                        }
                        div { class: "ts-modal-body",
                            label { class: "form-label", "Project / Task" }
                            select {
                                class: "form-select",
                                value: "{add_project}",
                                disabled: editing.read().is_some(),
                                onchange: move |e| add_project.set(e.value()),
                                for (id , label) in project_options.read().iter() {
                                    option { value: "{id}", "{label}" }
                                }
                            }
                            select {
                                class: "form-select",
                                value: "{add_task}",
                                disabled: editing.read().is_some(),
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
                            input {
                                class: "form-input",
                                "aria-label": "Start time",
                                placeholder: "Start time, e.g. 9:00 (optional)",
                                value: add_start().map(|m| horae_core::time_of_day::format(m as u16)).unwrap_or_default(),
                                oninput: move |e| {
                                    let v = e.value();
                                    let v = v.trim();
                                    if v.is_empty() {
                                        add_start.set(None);
                                    } else if let Some(m) = horae_core::time_of_day::parse(v) {
                                        add_start.set(Some(i32::from(m)));
                                    }
                                },
                            }
                            if let Some(err) = &*add_error.read() {
                                div { class: "ts-modal-error", "{err}" }
                            }
                            div { class: "ts-modal-actions",
                                // Harvest-style single primary: with no duration on
                                // today's column it starts a running timer; once a
                                // duration is typed (or on a past day / when editing)
                                // it saves a fixed entry.
                                button {
                                    class: "btn btn-primary",
                                    disabled: add_saving(),
                                    onclick: move |_| {
                                        let Some((project_id, task_id, notes)) = read_pt_notes.call(()) else {
                                            return;
                                        };
                                        let mut entries = entries;
                                        if timer_mode() {
                                            add_saving.set(true);
                                            add_error.set(None);
                                            spawn(async move {
                                                match server_fns::start_timer(project_id, task_id, notes).await {
                                                    Ok(_) => {
                                                        add_open.set(None);
                                                        entries.restart();
                                                    }
                                                    Err(e) => add_error
                                                        .set(Some(format!("Could not start timer: {e}"))),
                                                }
                                                add_saving.set(false);
                                            });
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
                                        let start_minute = *add_start.read();
                                        let editing_id = *editing.read();
                                        let billable = if editing_id.is_some() {
                                            edit_billable()
                                        } else {
                                            true
                                        };
                                        add_saving.set(true);
                                        add_error.set(None);
                                        spawn(async move {
                                            let result = persist_entry(
                                                editing_id, project_id, task_id, date, minutes,
                                                notes, billable, start_minute,
                                            )
                                            .await;
                                            match result {
                                                Ok(()) => {
                                                    add_open.set(None);
                                                    entries.restart();
                                                }
                                                Err(e) => add_error.set(Some(format!("Could not save: {e}"))),
                                            }
                                            add_saving.set(false);
                                        });
                                    },
                                    if add_saving() {
                                        "Saving…"
                                    } else if timer_mode() {
                                        "Start timer"
                                    } else {
                                        "Save entry"
                                    }
                                }
                                if editing.read().is_some() {
                                    button {
                                        class: "btn btn-danger",
                                        disabled: add_saving(),
                                        onclick: move |_| {
                                            let Some(id) = *editing.read() else {
                                                return;
                                            };
                                            let mut entries = entries;
                                            add_saving.set(true);
                                            add_error.set(None);
                                            spawn(async move {
                                                match server_fns::delete_time_entry(id.to_string()).await {
                                                    Ok(()) => {
                                                        add_open.set(None);
                                                        entries.restart();
                                                    }
                                                    Err(e) => {
                                                        add_error.set(Some(format!("Could not delete: {e}")))
                                                    }
                                                }
                                                add_saving.set(false);
                                            });
                                        },
                                        "Delete"
                                    }
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

            // Add-row picker: choose a project/task to add an empty grid row.
            if addrow_open() {
                div {
                    class: "modal-overlay",
                    onclick: move |_| addrow_open.set(false),
                    div {
                        class: "modal",
                        onclick: move |e| e.stop_propagation(),
                        div { class: "ts-modal-title", "Add a row" }
                        div { class: "ts-modal-body",
                            label { class: "form-label", "Project / Task" }
                            select {
                                class: "form-select",
                                value: "{addrow_project}",
                                onchange: move |e| addrow_project.set(e.value()),
                                for (id , label) in project_options.read().iter() {
                                    option { value: "{id}", "{label}" }
                                }
                            }
                            select {
                                class: "form-select",
                                value: "{addrow_task}",
                                onchange: move |e| addrow_task.set(e.value()),
                                for (id , label) in task_options.read().iter() {
                                    option { value: "{id}", "{label}" }
                                }
                            }
                            div { class: "ts-modal-actions",
                                button {
                                    class: "btn btn-primary",
                                    onclick: move |_| {
                                        let p = addrow_project.read().parse::<Uuid>();
                                        let t = addrow_task.read().parse::<Uuid>();
                                        if let (Ok(pid), Ok(tid)) = (p, t) {
                                            let key = (pid, tid);
                                            if !pending_rows.read().contains(&key) {
                                                pending_rows.write().push(key);
                                            }
                                        }
                                        addrow_open.set(false);
                                    },
                                    "Add row"
                                }
                                button {
                                    class: "btn btn-ghost",
                                    onclick: move |_| addrow_open.set(false),
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

/// A calendar event's pre-computed placement and labels, plus the entry it came
/// from so a click can open it for editing.
struct CalEvent {
    top: i32,
    height: i32,
    /// True when the entry has a start time (positioned at its hour); false when
    /// untimed (stacked from the top of the day).
    timed: bool,
    /// Column and column-count for laying overlapping timed blocks side by side.
    lane: i32,
    lanes: i32,
    project: String,
    task: String,
    duration: String,
    /// Start–end clock label (e.g. "9:00–10:30") for timed entries; empty when
    /// untimed.
    time_label: String,
    client: String,
    entry: TimeEntry,
}

/// Calendar grid pixels per hour.
const CAL_HOUR: i32 = 48;

/// Pointer Y (px within a day column) → snapped minutes since midnight.
fn cal_y_to_min(y: f64) -> i32 {
    horae_core::time_of_day::snap(
        (y * 60.0 / CAL_HOUR as f64) as i32,
        horae_core::time_of_day::SNAP_STEP,
    )
}

/// Assign overlapping timed entries to side-by-side lanes. Returns, per index of
/// `day` (parallel to the slice), the entry's lane and the number of lanes in its
/// overlap cluster; untimed entries get `(0, 1)`.
fn timed_lanes(day: &[TimeEntry]) -> (Vec<i32>, Vec<i32>) {
    let n = day.len();
    let mut lane_of = vec![0i32; n];
    let mut lanes_of = vec![1i32; n];

    // (index, start, end) for timed entries, sorted by start then end.
    let mut timed: Vec<(usize, i32, i32)> = day
        .iter()
        .enumerate()
        .filter_map(|(i, e)| e.start_minute.map(|s| (i, s, s + e.minutes)))
        .collect();
    timed.sort_by_key(|&(_, s, e)| (s, e));

    // Greedy: put each entry in the first lane free by its start time.
    let mut lane_end: Vec<i32> = Vec::new();
    for &(i, s, e) in &timed {
        let lane = match lane_end.iter().position(|&end| end <= s) {
            Some(l) => {
                lane_end[l] = e;
                l
            }
            None => {
                lane_end.push(e);
                lane_end.len() - 1
            }
        };
        lane_of[i] = lane as i32;
    }

    // Every entry in a maximal overlap run shares the run's lane count so all
    // stay the same width and none is hidden.
    let mut k = 0;
    while k < timed.len() {
        let mut j = k;
        let mut cluster_end = timed[k].2;
        let mut max_lane = 0i32;
        while j < timed.len() && timed[j].1 < cluster_end {
            cluster_end = cluster_end.max(timed[j].2);
            max_lane = max_lane.max(lane_of[timed[j].0]);
            j += 1;
        }
        let count = max_lane + 1;
        for &(i, _, _) in &timed[k..j] {
            lanes_of[i] = count;
        }
        k = j;
    }

    (lane_of, lanes_of)
}

/// What a calendar drag is doing: drawing a new slot, or moving/resizing an
/// existing timed entry.
#[derive(Clone, Copy, PartialEq)]
enum DragKind {
    Create,
    Move,
    Resize,
    /// Drag an untimed entry within its day's stack to reorder it.
    Reorder,
}

/// In-progress calendar drag (minutes are snapped). `day` is the column under the
/// pointer; for Move/Resize the target entry and its original span come along so
/// the release can reschedule it (or, if it didn't move, open it for editing).
#[derive(Clone)]
struct CalDrag {
    kind: DragKind,
    day: usize,
    /// Create: the slot's anchor. Move/Resize: the entry's original start minute.
    start_min: i32,
    cur_min: i32,
    /// Move: the pointer minute where the block was grabbed.
    grab_min: i32,
    /// Move/Resize target (None for Create).
    entry: Option<TimeEntry>,
    orig_dur: i32,
    orig_day: usize,
}

impl CalDrag {
    /// Move: the entry's new start, following the pointer while keeping the grab
    /// offset, clamped into the day. Shared by the commit and the live preview so
    /// the two can't drift.
    fn move_start(&self) -> i32 {
        (self.cur_min - (self.grab_min - self.start_min)).clamp(0, 1439)
    }

    /// Resize: the entry's new end — the bottom edge follows the pointer but stays
    /// at least one snap step below the start. Shared by the commit and preview.
    fn resize_end(&self) -> i32 {
        self.cur_min
            .max(self.start_min + i32::from(horae_core::time_of_day::MIN_DURATION))
    }
}

/// A calendar block's start–end clock label, e.g. "9:00–10:30". `end` is capped at
/// the end of day for display.
fn cal_time_label(start: i32, end: i32) -> String {
    format!(
        "{}–{}",
        horae_core::time_of_day::format(start as u16),
        horae_core::time_of_day::format(end.min(1440) as u16),
    )
}

/// A day's untimed (duration-only) entries in stacking order: by explicit
/// `sort_order`, then newest-first (the pre-reorder default). Shared by the
/// calendar placement and the reorder commit so both agree on the order.
fn untimed_ordered(day: &[TimeEntry]) -> Vec<TimeEntry> {
    let mut u: Vec<TimeEntry> = day
        .iter()
        .filter(|e| e.start_minute.is_none())
        .cloned()
        .collect();
    u.sort_by(|a, b| {
        a.sort_order
            .cmp(&b.sort_order)
            .then(b.created_at.cmp(&a.created_at))
    });
    u
}

#[expect(
    clippy::too_many_arguments,
    reason = "view renderer takes the week's data, display maps, and the add/edit/drag actions"
)]
fn render_calendar_view(
    by_day: &[Vec<TimeEntry>; 7],
    daily_totals: &[i32],
    visible_days: &[usize],
    week_start: NaiveDate,
    today: NaiveDate,
    labels: &CalLabels,
    mut cal_drag: Signal<Option<CalDrag>>,
    mut add_hint: Signal<Option<(usize, i32)>>,
    drag_commit: Callback<CalDrag>,
) -> Element {
    let today_off = today_offset(today, week_start);
    let col_class = |i: usize| day_col_class("ts-cal-col", today_off, i);
    let head_class = |i: usize| day_col_class("ts-cal-dayhead", today_off, i);

    // Place entries: timed ones (with a start time) at their hour; untimed ones
    // stacked from the top of the day by cumulative duration (Harvest does the
    // same for duration-only entries). Track the latest bottom so the grid is
    // tall enough to show every block.
    let mut day_events: Vec<Vec<CalEvent>> = Vec::with_capacity(7);
    // Per-column occupied minute ranges, so the "+ Add time" hint hides over
    // existing blocks and a click there edits instead of adding.
    let mut occupied: Vec<Vec<(i32, i32)>> = Vec::with_capacity(7);
    for day in by_day.iter() {
        // Lay out timed entries side by side where they overlap: greedily assign
        // each a lane, then give every entry in an overlap cluster the same
        // column count so none is hidden (SC-005). Keyed by index into `day`.
        let (lane_of, lanes_of) = timed_lanes(day);

        // Untimed blocks stack from the top in reorder-aware order; precompute
        // each one's top so the render loop can stay in the entries' own order.
        let mut untimed_top: HashMap<Uuid, i32> = HashMap::new();
        let mut cum = 0i32;
        for e in untimed_ordered(day) {
            untimed_top.insert(e.id, cum);
            cum += e.minutes;
        }
        let mut evs = Vec::new();
        let mut occ: Vec<(i32, i32)> = Vec::new();
        for (idx, e) in day.iter().enumerate() {
            let (top_min, timed, lane, lanes) = match e.start_minute {
                Some(sm) => (sm, true, lane_of[idx], lanes_of[idx].max(1)),
                None => (untimed_top.get(&e.id).copied().unwrap_or(0), false, 0, 1),
            };
            occ.push((top_min, top_min + e.minutes));
            let client = labels
                .clients
                .get(&e.project_id)
                .map(|(name, currency)| format!("{name} · {currency}"))
                .unwrap_or_default();
            let time_label = match e.start_minute {
                Some(sm) => cal_time_label(sm, sm + e.minutes),
                None => String::new(),
            };
            evs.push(CalEvent {
                top: top_min * CAL_HOUR / 60,
                height: (e.minutes * CAL_HOUR / 60).max(20),
                timed,
                lane,
                lanes,
                project: labels
                    .projects
                    .get(&e.project_id)
                    .cloned()
                    .unwrap_or_else(|| "Untitled".into()),
                task: labels.tasks.get(&e.task_id).cloned().unwrap_or_default(),
                duration: format_hm(e.minutes),
                time_label,
                client,
                entry: e.clone(),
            });
        }
        day_events.push(evs);
        occupied.push(occ);
    }
    // Show the full 24-hour day (the scroll container clips it); the grid scrolls
    // vertically and, on mount, jumps to the earliest block across the visible
    // days — or to the working hours (7am) when the range is empty.
    let max_hours = 24;
    let first_min = visible_days
        .iter()
        .flat_map(|&i| occupied.get(i).into_iter().flatten())
        .map(|&(s, _)| s)
        .min()
        .unwrap_or(7 * 60);
    let scroll_px = (first_min - 30).max(0) * CAL_HOUR / 60;

    // The grid spans a variable number of days; size the columns to match and
    // sum only the visible days for the header total.
    let n = visible_days.len().max(1);
    let shown_total: i32 = visible_days.iter().map(|&i| daily_totals[i]).sum();
    let total_label = if n == 1 { "Day total" } else { "Week total" };
    let grid_style = format!(
        "grid-template-columns: 56px repeat({n}, 1fr) 100px; min-width: {}px;",
        156 + n * 106
    );

    rsx! {
        div { class: "ts-cal",
            div {
                class: "ts-cal-scroll",
                // Open on the earliest block (or the working hours), not midnight.
                onmounted: move |evt: MountedEvent| {
                    spawn(async move {
                        let _ = evt
                            .data()
                            .scroll(
                                PixelsVector2D::new(0.0, f64::from(scroll_px)),
                                ScrollBehavior::Instant,
                            )
                            .await;
                    });
                },
                div { class: "ts-cal-head", style: "{grid_style}",
                    span {}
                    for i in visible_days.iter().copied() {
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
                        div { class: "ts-cal-weektot-label", "{total_label}" }
                        div { class: "ts-cal-weektot-value", "{format_hm(shown_total)}" }
                    }
                }

                div {
                    class: if cal_drag.read().is_some() { "ts-cal-grid dragging" } else { "ts-cal-grid" },
                    style: "{grid_style}",
                    onmouseleave: move |_| {
                        if cal_drag.read().is_some() {
                            cal_drag.set(None);
                        }
                        if add_hint.read().is_some() {
                            add_hint.set(None);
                        }
                    },
                    div { class: "ts-cal-rail",
                        for h in 0..max_hours {
                            div { class: "ts-cal-hour",
                                span { class: "ts-cal-hour-label", "{h + 1}hr" }
                            }
                        }
                    }
                    for i in visible_days.iter().copied() {
                        div {
                            class: "{col_class(i)}",
                            // Press-drag on an empty column draws a slot; release
                            // opens the entry form (a plain click has no start).
                            onmousedown: move |e: MouseEvent| {
                                let m = cal_y_to_min(e.element_coordinates().y);
                                cal_drag.set(Some(CalDrag {
                                    kind: DragKind::Create,
                                    day: i,
                                    start_min: m,
                                    cur_min: m,
                                    grab_min: m,
                                    entry: None,
                                    orig_dur: 0,
                                    orig_day: i,
                                }));
                            },
                            onmousemove: {
                                let occ = occupied[i].clone();
                                move |e: MouseEvent| {
                                    let m = cal_y_to_min(e.element_coordinates().y);
                                    if cal_drag.read().is_some() {
                                        cal_drag.with_mut(|d| {
                                            if let Some(d) = d {
                                                d.cur_min = m;
                                                // Move follows the pointer across days;
                                                // Reorder tracks the column so a drop on
                                                // another day snaps back.
                                                if matches!(d.kind, DragKind::Move | DragKind::Reorder) {
                                                    d.day = i;
                                                }
                                            }
                                        });
                                    } else {
                                        // Track the free slot under the cursor for the
                                        // "+ Add time" hint; hide it over a block.
                                        let free = !occ.iter().any(|&(lo, hi)| m >= lo && m < hi);
                                        let next = free.then_some((i, m));
                                        if *add_hint.read() != next {
                                            add_hint.set(next);
                                        }
                                    }
                                }
                            },
                            onmouseup: move |_| {
                                let drag = cal_drag.read().clone();
                                if let Some(d) = drag {
                                    cal_drag.set(None);
                                    drag_commit.call(d);
                                }
                            },
                            if let Some(d) = cal_drag.read().clone().filter(|d| d.day == i && d.kind == DragKind::Create) {
                                {
                                    let a = d.start_min.min(d.cur_min);
                                    let top = a * CAL_HOUR / 60;
                                    let h = ((d.cur_min - d.start_min).abs() * CAL_HOUR / 60).max(2);
                                    rsx! {
                                        div { class: "ts-cal-ghost", style: "top: {top}px; height: {h}px;" }
                                    }
                                }
                            }
                            // Cursor-following "+ Add time" over a free slot; a click
                            // there seeds a timed entry at that hour (drag_commit).
                            if let Some(top) = cal_drag
                                .read()
                                .is_none()
                                .then(|| *add_hint.read())
                                .flatten()
                                .filter(|&(hd, _)| hd == i)
                                .map(|(_, m)| m * CAL_HOUR / 60)
                            {
                                div { class: "ts-cal-add-hint", style: "top: {top}px;", "+ Add time" }
                            }
                            for ev in day_events[i].iter() {
                                {
                                // Live preview: while this entry is being moved or
                                // resized in its own column, drive its box from the
                                // in-progress drag so you can see it grow/shrink and
                                // read its new time (Create has its own ghost above).
                                let live = cal_drag.read().as_ref().and_then(|d| {
                                    if d.entry.as_ref().map(|e| e.id) != Some(ev.entry.id) {
                                        return None;
                                    }
                                    match d.kind {
                                        DragKind::Resize => Some((d.start_min, d.resize_end())),
                                        DragKind::Move if d.day == i => {
                                            let s = d.move_start();
                                            Some((s, s + d.orig_dur))
                                        }
                                        _ => None,
                                    }
                                });
                                let (top_px, height_px, time_label) = match live {
                                    Some((s, e)) => (
                                        s * CAL_HOUR / 60,
                                        ((e - s) * CAL_HOUR / 60).max(20),
                                        cal_time_label(s, e),
                                    ),
                                    None => (ev.top, ev.height, ev.time_label.clone()),
                                };
                                let ev_class = if live.is_some() {
                                    "ts-cal-event timed live"
                                } else if ev.timed {
                                    "ts-cal-event timed"
                                } else {
                                    "ts-cal-event"
                                };
                                rsx! {
                                div {
                                    class: "{ev_class}",
                                    style: "top: {top_px}px; height: {height_px}px; left: calc(4px + {ev.lane} * (100% - 8px) / {ev.lanes}); width: calc((100% - 8px) / {ev.lanes} - 2px); right: auto;",
                                    // Over a block there's no free slot — clear the
                                    // "+ Add time" hint (the column's mousemove can't
                                    // fire while the block captures the pointer).
                                    onmouseenter: move |_| {
                                        if add_hint.read().is_some() {
                                            add_hint.set(None);
                                        }
                                    },
                                    // Pressing a timed entry starts a move drag (its
                                    // body); a plain click with no move opens it for
                                    // editing. Untimed entries just open for editing.
                                    onmousedown: {
                                        let entry = ev.entry.clone();
                                        let timed = ev.timed;
                                        let start = ev.entry.start_minute.unwrap_or(0);
                                        let dur = ev.entry.minutes;
                                        move |e: MouseEvent| {
                                            e.stop_propagation();
                                            if timed {
                                                let off =
                                                    (e.element_coordinates().y * 60.0 / CAL_HOUR as f64) as i32;
                                                let g = start + off;
                                                cal_drag.set(Some(CalDrag {
                                                    kind: DragKind::Move,
                                                    day: i,
                                                    start_min: start,
                                                    cur_min: g,
                                                    grab_min: g,
                                                    entry: Some(entry.clone()),
                                                    orig_dur: dur,
                                                    orig_day: i,
                                                }));
                                            } else {
                                                // Untimed: drag to reorder within the
                                                // day's stack; cur_min tracks the drop.
                                                let m = cal_y_to_min(e.element_coordinates().y);
                                                cal_drag.set(Some(CalDrag {
                                                    kind: DragKind::Reorder,
                                                    day: i,
                                                    start_min: 0,
                                                    cur_min: m,
                                                    grab_min: m,
                                                    entry: Some(entry.clone()),
                                                    orig_dur: dur,
                                                    orig_day: i,
                                                }));
                                            }
                                        }
                                    },
                                    div { class: "ts-cal-ev-project",
                                        span { class: "ts-cal-ev-name", "{ev.project}" }
                                        span { class: "ts-cal-ev-dur", "{ev.duration}" }
                                    }
                                    if ev.timed {
                                        div { class: "ts-cal-ev-time", "{time_label}" }
                                    }
                                    if !ev.task.is_empty() {
                                        div { class: "ts-cal-ev-task", "{ev.task}" }
                                    }
                                    if !ev.client.is_empty() {
                                        div { class: "ts-cal-ev-client", "{ev.client}" }
                                    }
                                    if ev.timed {
                                        div {
                                            class: "ts-cal-resize",
                                            onmousedown: {
                                                let entry = ev.entry.clone();
                                                let start = ev.entry.start_minute.unwrap_or(0);
                                                let dur = ev.entry.minutes;
                                                move |e: MouseEvent| {
                                                    e.stop_propagation();
                                                    cal_drag.set(Some(CalDrag {
                                                        kind: DragKind::Resize,
                                                        day: i,
                                                        start_min: start,
                                                        cur_min: start + dur,
                                                        grab_min: start + dur,
                                                        entry: Some(entry.clone()),
                                                        orig_dur: dur,
                                                        orig_day: i,
                                                    }));
                                                }
                                            },
                                        }
                                    }
                                }
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

#[expect(
    clippy::too_many_arguments,
    reason = "view renderer takes the week's data, display maps, and the row actions"
)]
fn render_day_view(
    by_day: &[Vec<TimeEntry>; 7],
    daily_totals: &[i32],
    selected_offset: i64,
    select_day: Callback<i64>,
    project_names: &HashMap<Uuid, String>,
    task_names: &HashMap<Uuid, String>,
    open_edit: Callback<TimeEntry>,
    start_entry: Callback<TimeEntry>,
) -> Element {
    let offset = selected_offset.clamp(0, 6) as usize;
    let day_entries = &by_day[offset];
    let total = daily_totals[offset];

    rsx! {
        // Day strip: each day shows its own total and the viewed day is
        // underlined — Harvest presents the days this way here, not as tabs.
        div { class: "ts-daystrip",
            for i in 0i64..7 {
                {
                    let cls = if i == selected_offset { "ts-dayitem active" } else { "ts-dayitem" };
                    rsx! {
                        button {
                            class: "{cls}",
                            onclick: move |_| select_day.call(i),
                            span { class: "ts-dayitem-name", "{DAY_LABELS[i as usize]}" }
                            span { class: "ts-dayitem-total", "{format_hm(daily_totals[i as usize])}" }
                        }
                    }
                }
            }
            div { class: "ts-dayitem ts-weektotal",
                span { class: "ts-dayitem-name", "Week total" }
                span { class: "ts-dayitem-total", "{format_hm(daily_totals.iter().sum::<i32>())}" }
            }
        }

        div { class: "card",
            if day_entries.is_empty() {
                div { class: "ts-day-empty text-muted text-sm", "No entries for this day." }
            } else {
                div { class: "ts-day-list",
                    for entry in day_entries.iter() {
                        {
                            let proj = project_names.get(&entry.project_id).cloned().unwrap_or_else(|| entry.project_id.to_string());
                            let task = task_names.get(&entry.task_id).cloned().unwrap_or_else(|| "\u{2014}".into());
                            let note = entry.notes.clone().filter(|n| !n.trim().is_empty());
                            let running = entry.is_running;
                            let dur = entry.format_duration();
                            let e_start = entry.clone();
                            let e_edit = entry.clone();
                            rsx! {
                                div { class: "ts-day-entry",
                                    div { class: "ts-day-entry-main",
                                        div { class: "ts-day-entry-project", "{proj}" }
                                        div { class: "ts-day-entry-task", "{task}" }
                                        if let Some(n) = note {
                                            div { class: "ts-day-entry-notes", "{n}" }
                                        }
                                    }
                                    div { class: "ts-day-entry-side",
                                        if running {
                                            span { class: "badge badge-success", "Running" }
                                        } else {
                                            span { class: "ts-day-entry-dur text-mono", "{dur}" }
                                            button {
                                                class: "ts-day-action primary",
                                                onclick: move |_| start_entry.call(e_start.clone()),
                                                "Start"
                                            }
                                        }
                                        button {
                                            class: "ts-day-action",
                                            onclick: move |_| open_edit.call(e_edit.clone()),
                                            "Edit"
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

/// A single week-grid cell edit, committed when the input loses focus.
#[derive(Clone)]
struct CellEdit {
    project_id: Uuid,
    task_id: Uuid,
    day: NaiveDate,
    /// The entry already in the cell (update/delete), or `None` to create one.
    existing: Option<Uuid>,
    minutes: i32,
}

/// The actions the editable week grid dispatches back to the page.
#[derive(Clone, Copy)]
struct WeekActions {
    commit: Callback<CellEdit>,
    remove_row: Callback<(Uuid, Uuid)>,
    add_row: Callback<()>,
}

/// A week grid row's per-day minutes and the entry ids behind each day, so a cell
/// can update its entry (one id), create a new one (none), or fall back to a
/// read-only total when a day holds several entries for the same project/task.
#[derive(Default)]
struct RowAgg {
    mins: [i32; 7],
    ids: [Vec<Uuid>; 7],
}

#[expect(
    clippy::too_many_arguments,
    reason = "view renderer takes the week's data, display maps, pending rows, and grid actions"
)]
fn render_week_view(
    entries: &[TimeEntry],
    daily_totals: &[i32],
    week_start: NaiveDate,
    today: NaiveDate,
    project_names: &HashMap<Uuid, String>,
    task_names: &HashMap<Uuid, String>,
    pending: &[(Uuid, Uuid)],
    actions: WeekActions,
) -> Element {
    // Group by (project_id, task_id), tracking per-day minutes and entry ids,
    // preserving first-seen order. Rows added via "Add row" (no entries yet)
    // are appended so they render as empty, editable rows.
    let mut row_keys: Vec<(Uuid, Uuid)> = Vec::new();
    let mut row_map: HashMap<(Uuid, Uuid), RowAgg> = HashMap::new();
    for entry in entries {
        let offset = (entry.spent_date - week_start).num_days();
        if !(0..7).contains(&offset) {
            continue;
        }
        let row = row_map
            .entry((entry.project_id, entry.task_id))
            .or_insert_with(|| {
                row_keys.push((entry.project_id, entry.task_id));
                RowAgg::default()
            });
        row.mins[offset as usize] += entry.minutes;
        row.ids[offset as usize].push(entry.id);
    }
    for key in pending {
        if !row_map.contains_key(key) {
            row_keys.push(*key);
            row_map.insert(*key, RowAgg::default());
        }
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
                        let agg = &row_map[key];
                        let row_total: i32 = agg.mins.iter().sum();
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
                                        let mins = agg.mins[i];
                                        // A cell is editable when it holds at most one entry: type a
                                        // duration to create/update/clear it. Days with several entries
                                        // show a read-only total (edit them in the Day view).
                                        if agg.ids[i].len() <= 1 {
                                            let day = week_start + Duration::days(i as i64);
                                            let existing = agg.ids[i].first().copied();
                                            let val = if mins > 0 { format_hm(mins) } else { String::new() };
                                            let icls = value_cell_class("ts-cell-input", mins, today_off, i);
                                            rsx! {
                                                div { class: "ts-cell",
                                                    input {
                                                        class: "{icls}",
                                                        r#type: "text",
                                                        value: "{val}",
                                                        placeholder: "\u{2013}",
                                                        onchange: move |e| {
                                                            let raw = e.value();
                                                            let v = raw.trim();
                                                            let minutes = if v.is_empty() {
                                                                0
                                                            } else {
                                                                match horae_core::duration::parse(v) {
                                                                    Ok(m) if m <= 24 * 60 => m as i32,
                                                                    _ => return,
                                                                }
                                                            };
                                                            actions
                                                                .commit
                                                                .call(CellEdit { project_id: pid, task_id: tid, day, existing, minutes });
                                                        },
                                                    }
                                                }
                                            }
                                        } else {
                                            let cls = value_cell_class("ts-cell-box", mins, today_off, i);
                                            rsx! {
                                                div { class: "ts-cell",
                                                    div { class: "{cls}", title: "Multiple entries — edit in Day view", "{format_hm(mins)}" }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "ts-rowtotal", "{format_hm(row_total)}" }
                                div { class: "text-center",
                                    button {
                                        class: "ts-del",
                                        "aria-label": "Remove row",
                                        onclick: move |_| actions.remove_row.call((pid, tid)),
                                        "\u{00d7}"
                                    }
                                }
                            }
                        }
                    }
                }

                // Add a project/task row to fill in across the week.
                div { class: "ts-addrow-wrap",
                    button {
                        r#type: "button",
                        class: "ts-addrow",
                        onclick: move |_| actions.add_row.call(()),
                        span { class: "plus", "\u{ff0b}" }
                        "Add row"
                    }
                }

                // Footer: column totals
                div { class: "ts-row ts-foot",
                    div {}
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
