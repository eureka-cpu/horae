use std::collections::HashMap;

use dioxus::prelude::*;
use horae_core::duration::format_hhmm;
use horae_core::types::EntryState;
use uuid::Uuid;

use crate::components::avatar::{Avatar, first_initial};
use crate::components::badge::Badge;
use crate::components::controls::Segmented;
use crate::components::table::DataTable;
use crate::server_fns;

/// Minutes (aggregated, so `i64`) as `H:MM`, reusing the core formatter.
fn hhmm(minutes: i64) -> String {
    format_hhmm(minutes.max(0) as u32)
}

/// The `list_approvals` status argument for a Segmented label.
fn status_arg(label: &str) -> Option<String> {
    match label {
        "Approved" => Some("approved".to_string()),
        "All" => None,
        _ => Some("submitted".to_string()), // "Pending"
    }
}

/// Run a mutating server action, then clear/report the error and bump `refresh`
/// so the list re-loads. Shared by the per-row and bulk approve/reopen buttons.
fn spawn_action(
    fut: impl std::future::Future<Output = Result<(), ServerFnError>> + 'static,
    mut refresh: Signal<u32>,
    mut error: Signal<Option<String>>,
) {
    spawn(async move {
        match fut.await {
            Ok(()) => error.set(None),
            Err(e) => error.set(Some(e.to_string())),
        }
        refresh.set(refresh() + 1);
    });
}

#[component]
pub fn Approvals() -> Element {
    let me = use_resource(|| async move { server_fns::get_me().await });
    let mut status_label = use_signal(|| "Pending".to_string());
    let refresh = use_signal(|| 0u32);
    let action_error = use_signal(|| None::<String>);

    // Read the status signal inside the resource so a filter change re-loads.
    let approvals = use_resource(move || {
        let f = status_arg(&status_label.read());
        let _tick = *refresh.read();
        async move { server_fns::list_approvals(f).await }
    });

    let users = use_resource(|| async move { server_fns::list_users(false).await });
    let user_names: HashMap<Uuid, String> = users
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|us| us.iter().map(|u| (u.id, u.name.clone())).collect())
        .unwrap_or_default();

    let is_manager = me
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map(|u| u.is_manager_or_above())
        .unwrap_or(false);

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Approvals" }
            }

            if !is_manager {
                div { class: "card p-8 text-center",
                    p { class: "text-muted", "Manager or admin access is required to review approvals." }
                }
            } else {
                div { class: "mb-6",
                    Segmented {
                        items: vec!["Pending".to_string(), "Approved".to_string(), "All".to_string()],
                        active: status_label(),
                        onselect: move |v| status_label.set(v),
                    }
                }

                if let Some(err) = &*action_error.read() {
                    div { class: "alert alert-danger mb-4", "{err}" }
                }

                match &*approvals.read() {
                    None => rsx! { div { class: "text-muted text-sm", "Loading…" } },
                    Some(Err(e)) => rsx! { div { class: "alert alert-danger", "{e}" } },
                    Some(Ok(items)) if items.is_empty() => rsx! {
                        div { class: "card p-8 text-center",
                            p { class: "text-muted", "No approvals found." }
                        }
                    },
                    Some(Ok(items)) => {
                        let total: i64 = items.iter().map(|s| s.total_minutes).sum();
                        let billable: i64 = items.iter().map(|s| s.billable_minutes).sum();
                        let nonbill = (total - billable).max(0);
                        let bill_pct = if total > 0 { billable * 100 / total } else { 0 };
                        let nonbill_pct = if total > 0 { 100 - bill_pct } else { 0 };

                        let pending_ids: Vec<String> = items
                            .iter()
                            .filter(|s| s.approval.state == EntryState::Submitted)
                            .map(|s| s.approval.id.to_string())
                            .collect();
                        let pending_count = pending_ids.len();

                        rsx! {
                            // Total time (billable / non-billable). The design's second
                            // "Total expenses" card is omitted — Horae has no expenses.
                            div { class: "card flex items-center gap-8 mb-6",
                                div {
                                    div { class: "text-muted text-sm", "Total time" }
                                    div { class: "appr-total", "{hhmm(total)}" }
                                }
                                div { class: "flex-1 flex flex-col gap-2",
                                    div { class: "flex items-center gap-3",
                                        span { class: "appr-dot appr-dot-billable" }
                                        span { class: "flex-1 text-sm", "Billable" }
                                        span { class: "text-mono", "{hhmm(billable)}" }
                                        span { class: "appr-legend-pct", "({bill_pct}%)" }
                                    }
                                    div { class: "flex items-center gap-3",
                                        span { class: "appr-dot appr-dot-nonbillable" }
                                        span { class: "flex-1 text-sm", "Non-billable" }
                                        span { class: "text-mono", "{hhmm(nonbill)}" }
                                        span { class: "appr-legend-pct", "({nonbill_pct}%)" }
                                    }
                                }
                            }

                            DataTable {
                                table {
                                    thead {
                                        tr {
                                            th { "Teammate" }
                                            th { class: "text-right", "Hours" }
                                            th { class: "text-center", "Status" }
                                            th { "Submitted" }
                                            th { class: "text-right", "Action" }
                                        }
                                    }
                                    tbody {
                                        for s in items.iter() {
                                            {
                                                let a = s.approval.clone();
                                                let name = user_names
                                                    .get(&a.user_id)
                                                    .cloned()
                                                    .unwrap_or_else(|| a.user_id.to_string());
                                                let submitted = a.submitted_at.format("%d %b, %H:%M").to_string();
                                                let hours = hhmm(s.total_minutes);
                                                let is_pending = a.state == EntryState::Submitted;
                                                let aid = a.id.to_string();
                                                rsx! {
                                                    tr {
                                                        td {
                                                            div { class: "flex items-center gap-3",
                                                                Avatar { initials: first_initial(&name) }
                                                                span { class: "font-medium", "{name}" }
                                                            }
                                                        }
                                                        td { class: "text-mono text-right", "{hours}" }
                                                        td { class: "text-center",
                                                            match a.state {
                                                                EntryState::Submitted => rsx! { Badge { variant: "warning", "Awaiting" } },
                                                                EntryState::Approved => rsx! { Badge { variant: "success", "Approved" } },
                                                                other => rsx! { Badge { variant: "neutral", "{other}" } },
                                                            }
                                                        }
                                                        td { class: "text-mono text-sm text-muted", "{submitted}" }
                                                        td { class: "text-right",
                                                            if is_pending {
                                                                div { class: "flex gap-2 justify-end",
                                                                    button {
                                                                        class: "btn btn-secondary btn-sm",
                                                                        onclick: {
                                                                            let aid = aid.clone();
                                                                            move |_| spawn_action(
                                                                                { let aid = aid.clone(); async move { server_fns::reject_submission(aid).await } },
                                                                                refresh, action_error,
                                                                            )
                                                                        },
                                                                        "Reopen"
                                                                    }
                                                                    button {
                                                                        class: "btn btn-solid btn-sm",
                                                                        onclick: {
                                                                            let aid = aid.clone();
                                                                            move |_| spawn_action(
                                                                                { let aid = aid.clone(); async move { server_fns::approve_submission(aid).await.map(|_| ()) } },
                                                                                refresh, action_error,
                                                                            )
                                                                        },
                                                                        "Approve"
                                                                    }
                                                                }
                                                            } else {
                                                                span { class: "text-muted text-sm", "\u{2014}" }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if pending_count > 0 {
                                div { class: "flex items-center mt-5 gap-4",
                                    span { class: "text-muted text-sm",
                                        "{pending_count} timesheet(s) awaiting your approval"
                                    }
                                    div { class: "flex-1" }
                                    button {
                                        class: "btn btn-solid",
                                        onclick: move |_| spawn_action(
                                            { let ids = pending_ids.clone(); async move { server_fns::approve_submissions(ids).await.map(|_| ()) } },
                                            refresh, action_error,
                                        ),
                                        "Approve visible timesheets"
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
