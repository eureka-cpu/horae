// Server-only Axum handlers for CSV and XLSX export.
//
// These are plain Axum routes (not `#[server]` functions) because they
// return binary file data with custom Content-Type headers.

use axum::extract::Query;
use axum::response::IntoResponse;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ExportParams {
    pub from: String,
    pub to: String,
}

async fn fetch_entries(
    from: &str,
    to: &str,
) -> Vec<crate::models::DetailedReportRow> {
    let state = crate::state::global_state().await;
    sqlx::query_as::<_, crate::models::DetailedReportRow>(
        "SELECT te.spent_date, p.name AS project_name, t.name AS task_name,
                u.name AS user_name, te.minutes, te.rounded_minutes, te.billable, te.notes
         FROM time_entries te
         JOIN projects p ON te.project_id = p.id
         JOIN tasks t ON te.task_id = t.id
         JOIN users u ON te.user_id = u.id
         WHERE te.spent_date BETWEEN $1::date AND $2::date
         ORDER BY te.spent_date, p.name, t.name",
    )
    .bind(from)
    .bind(to)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
}

pub async fn export_csv(Query(params): Query<ExportParams>) -> impl IntoResponse {
    let entries = fetch_entries(&params.from, &params.to).await;

    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record([
        "Date",
        "Project",
        "Task",
        "User",
        "Hours",
        "Rounded Hours",
        "Billable",
        "Notes",
    ])
    .unwrap();

    for e in &entries {
        wtr.write_record(&[
            e.spent_date.to_string(),
            e.project_name.clone(),
            e.task_name.clone(),
            e.user_name.clone(),
            format!("{:.2}", e.minutes as f64 / 60.0),
            format!(
                "{:.2}",
                e.rounded_minutes.unwrap_or(e.minutes) as f64 / 60.0
            ),
            if e.billable { "Yes" } else { "No" }.into(),
            e.notes.clone().unwrap_or_default(),
        ])
        .unwrap();
    }

    let data = wtr.into_inner().unwrap();

    (
        [
            (axum::http::header::CONTENT_TYPE, "text/csv"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"timesheet.csv\"",
            ),
        ],
        data,
    )
}

pub async fn export_xlsx(Query(params): Query<ExportParams>) -> impl IntoResponse {
    let entries = fetch_entries(&params.from, &params.to).await;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    let headers = [
        "Date",
        "Project",
        "Task",
        "User",
        "Hours",
        "Rounded Hours",
        "Billable",
        "Notes",
    ];
    for (col, h) in headers.iter().enumerate() {
        worksheet.write_string(0, col as u16, *h).unwrap();
    }

    for (row, e) in entries.iter().enumerate() {
        let r = (row + 1) as u32;
        worksheet
            .write_string(r, 0, e.spent_date.to_string())
            .unwrap();
        worksheet.write_string(r, 1, &e.project_name).unwrap();
        worksheet.write_string(r, 2, &e.task_name).unwrap();
        worksheet.write_string(r, 3, &e.user_name).unwrap();
        worksheet
            .write_number(r, 4, e.minutes as f64 / 60.0)
            .unwrap();
        worksheet
            .write_number(
                r,
                5,
                e.rounded_minutes.unwrap_or(e.minutes) as f64 / 60.0,
            )
            .unwrap();
        worksheet
            .write_string(r, 6, if e.billable { "Yes" } else { "No" })
            .unwrap();
        worksheet
            .write_string(r, 7, e.notes.as_deref().unwrap_or(""))
            .unwrap();
    }

    let data = workbook.save_to_buffer().unwrap();

    (
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"timesheet.xlsx\"",
            ),
        ],
        data,
    )
}
