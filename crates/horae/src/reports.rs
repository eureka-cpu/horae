// Server-only Axum handlers for CSV and XLSX export.
//
// These are plain Axum routes (not `#[server]` functions) because they
// return binary file data with custom Content-Type headers.

use axum::extract::{Path, Query};
use axum::http::StatusCode;
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
) -> Result<Vec<crate::models::DetailedReportRow>, sqlx::Error> {
    let state = crate::state::global_state().await;
    sqlx::query_as!(
        crate::models::DetailedReportRow,
        r#"SELECT te.spent_date as "spent_date: chrono::NaiveDate",
                p.name AS project_name, t.name AS task_name,
                u.name AS user_name, te.minutes, te.rounded_minutes, te.billable, te.notes
         FROM time_entries te
         JOIN projects p ON te.project_id = p.id
         JOIN tasks t ON te.task_id = t.id
         JOIN users u ON te.user_id = u.id
         WHERE te.spent_date BETWEEN $1::date AND $2::date
         ORDER BY te.spent_date, p.name, t.name"#,
        from as &str,
        to as &str,
    )
    .fetch_all(&state.db)
    .await
}

pub async fn export_csv(
    Query(params): Query<ExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let entries = fetch_entries(&params.from, &params.to)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let data = wtr
        .into_inner()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "text/csv"),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"timesheet.csv\"",
            ),
        ],
        data,
    ))
}

pub async fn export_xlsx(
    Query(params): Query<ExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let entries = fetch_entries(&params.from, &params.to)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
        worksheet
            .write_string(0, col as u16, *h)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    for (row, e) in entries.iter().enumerate() {
        let r = (row + 1) as u32;
        worksheet
            .write_string(r, 0, e.spent_date.to_string())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_string(r, 1, &e.project_name)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_string(r, 2, &e.task_name)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_string(r, 3, &e.user_name)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_number(r, 4, e.minutes as f64 / 60.0)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_number(r, 5, e.rounded_minutes.unwrap_or(e.minutes) as f64 / 60.0)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_string(r, 6, if e.billable { "Yes" } else { "No" })
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_string(r, 7, e.notes.as_deref().unwrap_or(""))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let data = workbook
        .save_to_buffer()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((
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
    ))
}

// ── Invoice export ────────────────────────────────────────────────────────────

async fn fetch_invoice_lines(
    invoice_id: uuid::Uuid,
) -> Result<(crate::models::Invoice, Vec<crate::models::InvoiceLine>), StatusCode> {
    use horae_core::types::InvoiceStatus;

    let state = crate::state::global_state().await;
    let invoice = sqlx::query_as!(
        crate::models::Invoice,
        r#"SELECT id, org_id, client_id, number,
                  status as "status: InvoiceStatus",
                  issued_on as "issued_on: chrono::NaiveDate",
                  due_on as "due_on: chrono::NaiveDate",
                  currency, total_cents, notes,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM invoices WHERE id = $1"#,
        invoice_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let lines = sqlx::query_as!(
        crate::models::InvoiceLine,
        r#"SELECT id, invoice_id, time_entry_id, description,
                  minutes, rate_cents, amount_cents
           FROM invoice_line_items
           WHERE invoice_id = $1
           ORDER BY id"#,
        invoice_id,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((invoice, lines))
}

pub async fn export_invoice_csv(
    Path(invoice_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let (invoice, lines) = fetch_invoice_lines(invoice_id).await?;

    let mut wtr = csv::Writer::from_writer(vec![]);
    wtr.write_record(["Description", "Hours", "Rate", "Amount"])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for line in &lines {
        wtr.write_record(&[
            line.description.clone(),
            format!("{:.2}", line.minutes as f64 / 60.0),
            format!("{:.2}", line.rate_cents as f64 / 100.0),
            format!("{:.2}", line.amount_cents as f64 / 100.0),
        ])
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Total row
    wtr.write_record(&[
        "Total".to_string(),
        String::new(),
        String::new(),
        format!("{:.2}", invoice.total_cents as f64 / 100.0),
    ])
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let data = wtr
        .into_inner()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("invoice-{}.csv", invoice.number);
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "text/csv".to_string()),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        data,
    ))
}

pub async fn export_invoice_xlsx(
    Path(invoice_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let (invoice, lines) = fetch_invoice_lines(invoice_id).await?;

    let mut workbook = rust_xlsxwriter::Workbook::new();
    let worksheet = workbook.add_worksheet();

    let headers = ["Description", "Hours", "Rate", "Amount"];
    for (col, h) in headers.iter().enumerate() {
        worksheet
            .write_string(0, col as u16, *h)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    for (row, line) in lines.iter().enumerate() {
        let r = (row + 1) as u32;
        worksheet
            .write_string(r, 0, &line.description)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_number(r, 1, line.minutes as f64 / 60.0)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_number(r, 2, line.rate_cents as f64 / 100.0)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        worksheet
            .write_number(r, 3, line.amount_cents as f64 / 100.0)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Total row
    let total_row = (lines.len() + 1) as u32;
    worksheet
        .write_string(total_row, 0, "Total")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    worksheet
        .write_number(total_row, 3, invoice.total_cents as f64 / 100.0)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let data = workbook
        .save_to_buffer()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let filename = format!("invoice-{}.xlsx", invoice.number);
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        data,
    ))
}
