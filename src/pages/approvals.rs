use std::collections::HashMap;

use dioxus::prelude::*;
use uuid::Uuid;

use crate::server_fns;

#[component]
pub fn Approvals() -> Element {
    let me = use_resource(|| async move { server_fns::get_me().await });
    let mut status_filter = use_signal(|| Some("submitted".to_string()));
    let mut refresh_counter = use_signal(|| 0u32);
    let mut action_error = use_signal(|| None::<String>);

    let filter = status_filter.read().clone();
    let approvals = use_resource(move || {
        let f = filter.clone();
        let _tick = *refresh_counter.read();
        async move { server_fns::list_approvals(f).await }
    });

    let users = use_resource(|| async move { server_fns::list_users().await });

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

    let current_filter = status_filter.read().clone();

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Approvals" }
            }

            if !is_manager {
                div { class: "card", style: "padding: 2rem; text-align: center;",
                    p { class: "text-muted", "Manager or admin access is required to review approvals." }
                }
            } else {
                // Filter tabs
                div { style: "display: flex; gap: 0.25rem; margin-bottom: 1.5rem;",
                    button {
                        class: if current_filter.as_deref() == Some("submitted") { "btn btn-primary" } else { "btn btn-secondary" },
                        onclick: move |_| status_filter.set(Some("submitted".to_string())),
                        "Pending"
                    }
                    button {
                        class: if current_filter.as_deref() == Some("approved") { "btn btn-primary" } else { "btn btn-secondary" },
                        onclick: move |_| status_filter.set(Some("approved".to_string())),
                        "Approved"
                    }
                    button {
                        class: if current_filter.is_none() { "btn btn-primary" } else { "btn btn-secondary" },
                        onclick: move |_| status_filter.set(None),
                        "All"
                    }
                }

                if let Some(err) = &*action_error.read() {
                    div { class: "alert alert-danger", style: "margin-bottom: 1rem;", "{err}" }
                }

                match &*approvals.read() {
                    None => rsx! { div { class: "text-muted text-sm", "Loading..." } },
                    Some(Err(e)) => rsx! { div { class: "alert alert-danger", "{e}" } },
                    Some(Ok(items)) if items.is_empty() => rsx! {
                        div { class: "card", style: "padding: 2rem; text-align: center;",
                            p { class: "text-muted", "No approvals found." }
                        }
                    },
                    Some(Ok(items)) => rsx! {
                        div { class: "card",
                            div { class: "table-container",
                                table {
                                    thead {
                                        tr {
                                            th { "Team Member" }
                                            th { "Period" }
                                            th { "Submitted" }
                                            th { "Status" }
                                            th { "Actions" }
                                        }
                                    }
                                    tbody {
                                        for approval in items.iter() {
                                            {
                                                let name = user_names
                                                    .get(&approval.user_id)
                                                    .cloned()
                                                    .unwrap_or_else(|| approval.user_id.to_string());
                                                let period = format!(
                                                    "{} - {}",
                                                    approval.period_start.format("%b %d"),
                                                    approval.period_end.format("%b %d, %Y")
                                                );
                                                let submitted = approval.submitted_at.format("%b %d, %Y %H:%M").to_string();
                                                let state = approval.state.clone();
                                                let aid = approval.id.to_string();
                                                let aid2 = aid.clone();
                                                let is_pending = state == "submitted";
                                                rsx! {
                                                    tr {
                                                        td { "{name}" }
                                                        td { class: "text-mono", "{period}" }
                                                        td { class: "text-mono", style: "font-size: 0.85rem;", "{submitted}" }
                                                        td {
                                                            match state.as_str() {
                                                                "submitted" => rsx! { span { class: "badge badge-warning", "Pending" } },
                                                                "approved" => rsx! { span { class: "badge badge-success", "Approved" } },
                                                                other => rsx! { span { class: "badge badge-neutral", "{other}" } },
                                                            }
                                                        }
                                                        td {
                                                            if is_pending {
                                                                div { style: "display: flex; gap: 0.5rem;",
                                                                    button {
                                                                        class: "btn btn-primary",
                                                                        style: "padding: 0.25rem 0.75rem; font-size: 0.85rem;",
                                                                        onclick: {
                                                                            let aid = aid.clone();
                                                                            move |_| {
                                                                                let aid = aid.clone();
                                                                                let mut rc = refresh_counter;
                                                                                let mut ae = action_error;
                                                                                spawn(async move {
                                                                                    match server_fns::approve_submission(aid).await {
                                                                                        Ok(_) => ae.set(None),
                                                                                        Err(e) => ae.set(Some(e.to_string())),
                                                                                    }
                                                                                    rc.set(rc() + 1);
                                                                                });
                                                                            }
                                                                        },
                                                                        "Approve"
                                                                    }
                                                                    button {
                                                                        class: "btn btn-danger",
                                                                        style: "padding: 0.25rem 0.75rem; font-size: 0.85rem;",
                                                                        onclick: {
                                                                            let aid2 = aid2.clone();
                                                                            move |_| {
                                                                                let aid2 = aid2.clone();
                                                                                let mut rc = refresh_counter;
                                                                                let mut ae = action_error;
                                                                                spawn(async move {
                                                                                    match server_fns::reject_submission(aid2).await {
                                                                                        Ok(_) => ae.set(None),
                                                                                        Err(e) => ae.set(Some(e.to_string())),
                                                                                    }
                                                                                    rc.set(rc() + 1);
                                                                                });
                                                                            }
                                                                        },
                                                                        "Reject"
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
                        }
                    },
                }
            }
        }
    }
}
