use chrono::Datelike;
use dioxus::prelude::*;

use crate::components::badge::Badge;
use crate::components::table::DataTable;
use crate::server_fns;

/// Minutes as decimal hours — the report/export convention (e.g. 90 → "1.50").
fn hours(minutes: i64) -> String {
    format!("{:.2}", minutes as f64 / 60.0)
}

#[component]
pub fn Reports() -> Element {
    let today = chrono::Utc::now().date_naive();
    let month_start = today.with_day(1).unwrap_or(today);

    let mut from_date = use_signal(move || month_start.to_string());
    let mut to_date = use_signal(move || today.to_string());
    let mut group_by = use_signal(|| "project".to_string());
    let mut active_tab = use_signal(|| "time".to_string());

    // Read the signals inside each resource so a filter change re-loads.
    let summary = use_resource(move || {
        let f = from_date.read().clone();
        let t = to_date.read().clone();
        let g = group_by.read().clone();
        async move { server_fns::report_time(f, t, g).await }
    });
    let detailed = use_resource(move || {
        let f = from_date.read().clone();
        let t = to_date.read().clone();
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

    let tab = active_tab.read().clone();

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Reports" }
                div { class: "page-actions",
                    a { class: "btn btn-secondary", href: "{export_csv_url}", "Export CSV" }
                    a { class: "btn btn-secondary", href: "{export_xlsx_url}", "Export XLSX" }
                }
            }

            div { class: "card mb-6",
                div { class: "flex gap-4 items-end flex-wrap",
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
                            class: "form-select",
                            value: "{group_by}",
                            oninput: move |e| group_by.set(e.value()),
                            option { value: "project", "Project" }
                            option { value: "task", "Task" }
                            option { value: "person", "Person" }
                        }
                    }
                }
            }

            div { class: "report-tabs",
                button {
                    class: if tab == "time" { "report-tab active" } else { "report-tab" },
                    onclick: move |_| active_tab.set("time".into()),
                    "Time"
                }
                button {
                    class: if tab == "detailed" { "report-tab active" } else { "report-tab" },
                    onclick: move |_| active_tab.set("detailed".into()),
                    "Detailed time"
                }
            }

            if tab == "time" {
                match &*summary.read() {
                    None => rsx! { div { class: "text-muted text-sm", "Loading…" } },
                    Some(Err(e)) => rsx! { div { class: "alert alert-danger", "{e}" } },
                    Some(Ok(rows)) if rows.is_empty() => rsx! {
                        div { class: "card p-8 text-center",
                            p { class: "text-muted", "No time tracked in this range." }
                        }
                    },
                    Some(Ok(rows)) => {
                        let grand_total: i64 = rows.iter().map(|r| r.total_minutes).sum();
                        let grand_rounded: i64 = rows.iter().map(|r| r.rounded_minutes).sum();
                        let grand_billable: i64 = rows.iter().map(|r| r.billable_minutes).sum();
                        rsx! {
                            DataTable {
                                table {
                                    thead {
                                        tr {
                                            th { "Group" }
                                            th { class: "text-right", "Total hours" }
                                            th { class: "text-right", "Rounded" }
                                            th { class: "text-right", "Billable" }
                                        }
                                    }
                                    tbody {
                                        for row in rows.iter() {
                                            tr { key: "{row.label}",
                                                td { "{row.label}" }
                                                td { class: "text-mono text-right", "{hours(row.total_minutes)}" }
                                                td { class: "text-mono text-right", "{hours(row.rounded_minutes)}" }
                                                td { class: "text-mono text-right", "{hours(row.billable_minutes)}" }
                                            }
                                        }
                                        tr { class: "report-total-row",
                                            td { "Total" }
                                            td { class: "text-mono text-right", "{hours(grand_total)}" }
                                            td { class: "text-mono text-right", "{hours(grand_rounded)}" }
                                            td { class: "text-mono text-right", "{hours(grand_billable)}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if tab == "detailed" {
                match &*detailed.read() {
                    None => rsx! { div { class: "text-muted text-sm", "Loading…" } },
                    Some(Err(e)) => rsx! { div { class: "alert alert-danger", "{e}" } },
                    Some(Ok(entries)) if entries.is_empty() => rsx! {
                        div { class: "card p-8 text-center",
                            p { class: "text-muted", "No entries in this range." }
                        }
                    },
                    Some(Ok(entries)) => rsx! {
                        DataTable {
                            table {
                                thead {
                                    tr {
                                        th { "Date" }
                                        th { "Project" }
                                        th { "Task" }
                                        th { "Teammate" }
                                        th { class: "text-right", "Hours" }
                                        th { class: "text-right", "Rounded" }
                                        th { class: "text-center", "Billable" }
                                        th { "Notes" }
                                    }
                                }
                                tbody {
                                    for (i, e) in entries.iter().enumerate() {
                                        tr { key: "{i}",
                                            td { class: "text-mono", "{e.spent_date}" }
                                            td { "{e.project_name}" }
                                            td { "{e.task_name}" }
                                            td { "{e.user_name}" }
                                            td { class: "text-mono text-right", "{hours(e.minutes as i64)}" }
                                            td { class: "text-mono text-right",
                                                "{hours(e.rounded_minutes.unwrap_or(e.minutes) as i64)}"
                                            }
                                            td { class: "text-center",
                                                if e.billable {
                                                    Badge { variant: "success", "Yes" }
                                                } else {
                                                    Badge { variant: "neutral", "No" }
                                                }
                                            }
                                            td { "{e.notes.as_deref().unwrap_or(\"\u{2014}\")}" }
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
