use chrono::Datelike;
use dioxus::prelude::*;

use crate::server_fns;

#[component]
pub fn Reports() -> Element {
    // Default to current month range
    let today = chrono::Utc::now().date_naive();
    let month_start = today.with_day(1).unwrap_or(today);

    let mut from_date = use_signal(move || month_start.to_string());
    let mut to_date = use_signal(move || today.to_string());
    let mut group_by = use_signal(|| "project".to_string());
    let mut active_tab = use_signal(|| "summary".to_string());

    let from_val = from_date.read().clone();
    let to_val = to_date.read().clone();
    let group_val = group_by.read().clone();

    let summary = use_resource(move || {
        let f = from_val.clone();
        let t = to_val.clone();
        let g = group_val.clone();
        async move { server_fns::report_time(f, t, g).await }
    });

    let from_val2 = from_date.read().clone();
    let to_val2 = to_date.read().clone();

    let detailed = use_resource(move || {
        let f = from_val2.clone();
        let t = to_val2.clone();
        async move { server_fns::report_detailed(f, t).await }
    });

    let export_csv_url = format!(
        "/api/reports/export/csv?from={}&to={}",
        from_date.read(),
        to_date.read()
    );
    let export_xlsx_url = format!(
        "/api/reports/export/xlsx?from={}&to={}",
        from_date.read(),
        to_date.read()
    );

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Reports" }
                div { class: "page-actions", style: "display: flex; gap: 0.5rem;",
                    a {
                        class: "btn btn-secondary",
                        href: "{export_csv_url}",
                        "Export CSV"
                    }
                    a {
                        class: "btn btn-secondary",
                        href: "{export_xlsx_url}",
                        "Export XLSX"
                    }
                }
            }

            // Filters
            div { class: "card",
                div { style: "display: flex; gap: 1rem; align-items: flex-end; flex-wrap: wrap;",
                    div { class: "form-group",
                        label { class: "form-label", "From" }
                        input {
                            class: "form-input",
                            r#type: "date",
                            value: "{from_date}",
                            oninput: move |e| from_date.set(e.value()),
                        }
                    }
                    div { class: "form-group",
                        label { class: "form-label", "To" }
                        input {
                            class: "form-input",
                            r#type: "date",
                            value: "{to_date}",
                            oninput: move |e| to_date.set(e.value()),
                        }
                    }
                    div { class: "form-group",
                        label { class: "form-label", "Group by" }
                        select {
                            class: "form-input",
                            value: "{group_by}",
                            oninput: move |e| group_by.set(e.value()),
                            option { value: "project", "Project" }
                            option { value: "task", "Task" }
                            option { value: "person", "Person" }
                        }
                    }
                }
            }

            // Tabs
            div { class: "mt-4", style: "display: flex; gap: 0; border-bottom: 1px solid var(--color-border);",
                button {
                    class: "btn btn-ghost",
                    style: if *active_tab.read() == "summary" {
                        "border-bottom: 2px solid var(--color-primary); border-radius: 0; color: var(--color-primary);"
                    } else {
                        "border-bottom: 2px solid transparent; border-radius: 0; color: var(--color-text-secondary);"
                    },
                    onclick: move |_| active_tab.set("summary".into()),
                    "Summary"
                }
                button {
                    class: "btn btn-ghost",
                    style: if *active_tab.read() == "detailed" {
                        "border-bottom: 2px solid var(--color-primary); border-radius: 0; color: var(--color-primary);"
                    } else {
                        "border-bottom: 2px solid transparent; border-radius: 0; color: var(--color-text-secondary);"
                    },
                    onclick: move |_| active_tab.set("detailed".into()),
                    "Detailed"
                }
            }

            // Summary tab
            if *active_tab.read() == "summary" {
                div { class: "card mt-4",
                    match &*summary.read() {
                        Some(Ok(rows)) => {
                            let rows = rows.clone();
                            let grand_total: i64 = rows.iter().map(|r| r.total_minutes).sum();
                            let grand_rounded: i64 = rows.iter().map(|r| r.rounded_minutes).sum();
                            let grand_billable: i64 = rows.iter().map(|r| r.billable_minutes).sum();
                            rsx! {
                                div { class: "table-container",
                                    table {
                                        thead {
                                            tr {
                                                th { "Group" }
                                                th { "Total Hours" }
                                                th { "Rounded Hours" }
                                                th { "Billable Hours" }
                                            }
                                        }
                                        tbody {
                                            for row in rows.iter() {
                                                tr { key: "{row.label}",
                                                    td { "{row.label}" }
                                                    td { class: "text-mono",
                                                        "{row.total_minutes as f64 / 60.0:.2}"
                                                    }
                                                    td { class: "text-mono",
                                                        "{row.rounded_minutes as f64 / 60.0:.2}"
                                                    }
                                                    td { class: "text-mono",
                                                        "{row.billable_minutes as f64 / 60.0:.2}"
                                                    }
                                                }
                                            }
                                            tr { style: "font-weight: 600; border-top: 2px solid var(--color-border);",
                                                td { "Total" }
                                                td { class: "text-mono",
                                                    "{grand_total as f64 / 60.0:.2}"
                                                }
                                                td { class: "text-mono",
                                                    "{grand_rounded as f64 / 60.0:.2}"
                                                }
                                                td { class: "text-mono",
                                                    "{grand_billable as f64 / 60.0:.2}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "alert alert-danger", "{e}" }
                        },
                        None => rsx! {
                            div { class: "text-muted text-sm", "Loading..." }
                        },
                    }
                }
            }

            // Detailed tab
            if *active_tab.read() == "detailed" {
                div { class: "card mt-4",
                    match &*detailed.read() {
                        Some(Ok(entries)) => {
                            let entries = entries.clone();
                            rsx! {
                                div { class: "table-container",
                                    table {
                                        thead {
                                            tr {
                                                th { "Date" }
                                                th { "Project" }
                                                th { "Task" }
                                                th { "User" }
                                                th { "Hours" }
                                                th { "Rounded" }
                                                th { "Billable" }
                                                th { "Notes" }
                                            }
                                        }
                                        tbody {
                                            for (i, entry) in entries.iter().enumerate() {
                                                tr { key: "{i}",
                                                    td { class: "text-mono", "{entry.spent_date}" }
                                                    td { "{entry.project_name}" }
                                                    td { "{entry.task_name}" }
                                                    td { "{entry.user_name}" }
                                                    td { class: "text-mono",
                                                        "{entry.minutes as f64 / 60.0:.2}"
                                                    }
                                                    td { class: "text-mono",
                                                        "{entry.rounded_minutes.unwrap_or(entry.minutes) as f64 / 60.0:.2}"
                                                    }
                                                    td {
                                                        if entry.billable {
                                                            span { class: "badge badge-info", "Yes" }
                                                        } else {
                                                            span { class: "badge badge-neutral", "No" }
                                                        }
                                                    }
                                                    td { "{entry.notes.as_deref().unwrap_or(\"-\")}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Some(Err(e)) => rsx! {
                            div { class: "alert alert-danger", "{e}" }
                        },
                        None => rsx! {
                            div { class: "text-muted text-sm", "Loading..." }
                        },
                    }
                }
            }
        }
    }
}
