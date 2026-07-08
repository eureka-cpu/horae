use std::collections::HashMap;

use dioxus::prelude::*;
use tracing::error;
use uuid::Uuid;

use crate::server_fns;

#[component]
pub fn TimerWidget() -> Element {
    // Fetch current running timer on mount
    let mut timer_resource = use_resource(|| async move {
        server_fns::get_current_timer().await
    });

    // Fetch projects and tasks for the pickers
    let projects = use_resource(|| async move {
        server_fns::list_projects(None, Some(true)).await
    });
    let tasks = use_resource(|| async move {
        server_fns::list_tasks(None).await
    });

    // Form state for project/task selection
    let mut selected_project = use_signal(String::new);
    let mut selected_task = use_signal(String::new);

    // Build project name lookup
    let project_names: HashMap<Uuid, String> = projects
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|ps| ps.iter().map(|p| (p.id, p.name.clone())).collect())
        .unwrap_or_default();

    // Determine if a timer is currently running
    let current_timer = timer_resource
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .flatten();

    let is_running = current_timer.is_some();

    // Compute elapsed time display
    let (hours, minutes, seconds) = if let Some(ref entry) = current_timer {
        if let Some(started_at) = entry.started_at {
            let now = chrono::Utc::now();
            let elapsed_secs = (now - started_at).num_seconds().max(0) as u64;
            let total = elapsed_secs + (entry.minutes as u64 * 60);
            (total / 3600, (total % 3600) / 60, total % 60)
        } else {
            let total = entry.minutes as u64 * 60;
            (total / 3600, (total % 3600) / 60, 0u64)
        }
    } else {
        (0u64, 0u64, 0u64)
    };

    // Project name for the running timer
    let running_project_name = current_timer
        .as_ref()
        .and_then(|entry| project_names.get(&entry.project_id))
        .cloned();

    let handle_start = move |_| {
        let proj = selected_project.read().clone();
        let task = selected_task.read().clone();
        spawn(async move {
            if proj.is_empty() || task.is_empty() {
                return; // need both selected
            }
            match server_fns::start_timer(proj, task, None).await {
                Ok(_) => {
                    timer_resource.restart();
                }
                Err(e) => {
                    error!("Start timer error: {e}");
                }
            }
        });
    };

    let entry_id_for_stop = current_timer.as_ref().map(|e| e.id.to_string());
    let handle_stop = move |_| {
        if let Some(eid) = entry_id_for_stop.clone() {
            spawn(async move {
                match server_fns::stop_timer(eid).await {
                    Ok(_) => {
                        timer_resource.restart();
                    }
                    Err(e) => {
                        error!("Stop timer error: {e}");
                    }
                }
            });
        }
    };

    rsx! {
        div { class: if is_running { "timer-widget timer-running" } else { "timer-widget" },
            div { class: "timer-display text-mono",
                "{hours:02}:{minutes:02}:{seconds:02}"
            }

            if is_running {
                div {
                    p { class: "text-sm",
                        {running_project_name.unwrap_or_else(|| "Unknown project".into())}
                    }
                }
            } else {
                div { class: "timer-pickers", style: "display: flex; gap: 0.5rem; align-items: center;",
                    select {
                        class: "form-input",
                        value: "{selected_project}",
                        oninput: move |e| selected_project.set(e.value()),
                        option { value: "", "Select project..." }
                        {projects.read().as_ref().and_then(|r| r.as_ref().ok()).map(|ps| {
                            rsx! {
                                for p in ps.iter() {
                                    option { value: "{p.id}", "{p.name}" }
                                }
                            }
                        })}
                    }
                    select {
                        class: "form-input",
                        value: "{selected_task}",
                        oninput: move |e| selected_task.set(e.value()),
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
            }

            div { style: "margin-left: auto;",
                if is_running {
                    button {
                        class: "btn btn-danger",
                        onclick: handle_stop,
                        "Stop"
                    }
                } else {
                    button {
                        class: "btn btn-primary",
                        onclick: handle_start,
                        "Start"
                    }
                }
            }
        }
    }
}
