use std::collections::HashMap;

use dioxus::prelude::*;
use tracing::error;
use uuid::Uuid;

use crate::server_fns;

/// The sidebar timer. Idle shows a "Start timer" button; picking a project/task
/// starts a running timer; while running it shows the live elapsed time, the
/// project, and a Stop button. It lives in the sidebar so it's reachable from
/// every page (Harvest-style), not just the timesheet.
#[component]
pub fn TimerWidget() -> Element {
    // A 1s tick re-renders this component so the running display counts up.
    let mut tick = use_signal(|| 0u64);
    let _tick = *tick.read();
    use_hook(|| {
        spawn(async move {
            loop {
                #[cfg(feature = "web")]
                gloo_timers::future::TimeoutFuture::new(1_000).await;
                #[cfg(feature = "server")]
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                tick += 1;
            }
        });
    });

    let mut timer_resource = use_resource(|| async move { server_fns::get_current_timer().await });
    let projects = use_resource(|| async move { server_fns::list_projects(None, false).await });

    let mut picking = use_signal(|| false);
    let mut selected_project = use_signal(String::new);
    let mut selected_task = use_signal(String::new);

    // Tasks narrow to the picked project, falling back to all tasks.
    let tasks = use_resource(move || {
        let proj = selected_project.read().clone();
        async move {
            if proj.is_empty() {
                server_fns::list_tasks().await
            } else {
                server_fns::list_project_tasks(proj).await
            }
        }
    });

    let project_names: HashMap<Uuid, String> = projects
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|ps| ps.iter().map(|p| (p.id, p.name.clone())).collect())
        .unwrap_or_default();

    let current_timer = timer_resource
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .flatten();
    let is_running = current_timer.is_some();

    // Elapsed = time since it started, plus any minutes already banked on the entry.
    let (hours, minutes, seconds) = match current_timer
        .as_ref()
        .and_then(|e| e.started_at.map(|s| (e, s)))
    {
        Some((entry, started_at)) => {
            let elapsed = (chrono::Utc::now() - started_at).num_seconds().max(0) as u64;
            let total = elapsed + entry.minutes as u64 * 60;
            (total / 3600, (total % 3600) / 60, total % 60)
        }
        None => (0, 0, 0),
    };

    let running_project_name = current_timer
        .as_ref()
        .and_then(|e| project_names.get(&e.project_id))
        .cloned();

    let handle_start = move |_| {
        let proj = selected_project.read().clone();
        let task = selected_task.read().clone();
        if proj.is_empty() || task.is_empty() {
            return;
        }
        spawn(async move {
            match server_fns::start_timer(proj, task, None).await {
                Ok(_) => {
                    picking.set(false);
                    timer_resource.restart();
                }
                Err(e) => error!("Start timer error: {e}"),
            }
        });
    };

    let entry_id_for_stop = current_timer.as_ref().map(|e| e.id.to_string());
    let handle_stop = move |_| {
        if let Some(eid) = entry_id_for_stop.clone() {
            spawn(async move {
                match server_fns::stop_timer(eid).await {
                    Ok(_) => timer_resource.restart(),
                    Err(e) => error!("Stop timer error: {e}"),
                }
            });
        }
    };

    rsx! {
        div { class: "sidebar-timer-wrap",
            if is_running {
                div { class: "sidebar-timer-live",
                    div { class: "sidebar-timer-time", "{hours:02}:{minutes:02}:{seconds:02}" }
                    div { class: "sidebar-timer-proj",
                        {running_project_name.unwrap_or_else(|| "Running".into())}
                    }
                    button { class: "sidebar-timer-stop", onclick: handle_stop, "Stop" }
                }
            } else if picking() {
                div { class: "sidebar-timer-form",
                    select {
                        value: "{selected_project}",
                        oninput: move |e| {
                            selected_project.set(e.value());
                            selected_task.set(String::new());
                        },
                        option { value: "", "Select project…" }
                        {projects.read().as_ref().and_then(|r| r.as_ref().ok()).map(|ps| rsx! {
                            for p in ps.iter() {
                                option { value: "{p.id}", "{p.name}" }
                            }
                        })}
                    }
                    select {
                        value: "{selected_task}",
                        oninput: move |e| selected_task.set(e.value()),
                        option { value: "", "Select task…" }
                        {tasks.read().as_ref().and_then(|r| r.as_ref().ok()).map(|ts| rsx! {
                            for t in ts.iter() {
                                option { value: "{t.id}", "{t.name}" }
                            }
                        })}
                    }
                    div { class: "sidebar-timer-form-actions",
                        button { class: "btn btn-primary", onclick: handle_start, "Start" }
                        button {
                            class: "btn btn-ghost",
                            onclick: move |_| picking.set(false),
                            "Cancel"
                        }
                    }
                }
            } else {
                button {
                    class: "sidebar-timer",
                    onclick: move |_| picking.set(true),
                    span { class: "sidebar-timer-icon" }
                    span { class: "sidebar-timer-label", "Start timer" }
                }
            }
        }
    }
}
