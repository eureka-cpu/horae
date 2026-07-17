//! Invoice server functions.

use super::*;

// ── Invoices ──────────────────────────────────────────────────────────────────

#[server]
pub async fn list_invoices(status: Option<String>) -> Result<Vec<Invoice>, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;

    let status_filter: Option<InvoiceStatus> = status
        .as_deref()
        .map(|s| {
            s.parse::<InvoiceStatus>()
                .map_err(|_| server_err("Invalid status"))
        })
        .transpose()?;

    let invoices = sqlx::query_as!(
        Invoice,
        r#"SELECT id, org_id, client_id, number,
                  status as "status: InvoiceStatus",
                  issued_on as "issued_on: chrono::NaiveDate",
                  due_on as "due_on: chrono::NaiveDate",
                  currency, total_cents, notes,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM invoices
           WHERE org_id = $1
             AND ($2::invoice_status IS NULL OR status = $2)
           ORDER BY created_at DESC"#,
        manager.org_id,
        status_filter as Option<InvoiceStatus>,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(invoices)
}

#[server]
pub async fn get_invoice(invoice_id: String) -> Result<InvoiceWithLines, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let id: uuid::Uuid = invoice_id
        .parse()
        .map_err(|_| server_err("Invalid invoice_id"))?;

    let invoice = sqlx::query_as!(
        Invoice,
        r#"SELECT id, org_id, client_id, number,
                  status as "status: InvoiceStatus",
                  issued_on as "issued_on: chrono::NaiveDate",
                  due_on as "due_on: chrono::NaiveDate",
                  currency, total_cents, notes,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM invoices
           WHERE id = $1 AND org_id = $2"#,
        id,
        manager.org_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Invoice not found".into(),
        code: NOT_FOUND,
        details: None,
    })?;

    let lines = sqlx::query_as!(
        InvoiceLine,
        r#"SELECT id, invoice_id, time_entry_id, description,
                  minutes, rate_cents, amount_cents
           FROM invoice_line_items
           WHERE invoice_id = $1
           ORDER BY id"#,
        id,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(InvoiceWithLines { invoice, lines })
}

#[server]
pub async fn generate_invoice(
    client_id: String,
    period_from: String,
    period_to: String,
) -> Result<InvoiceWithLines, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let client_id: uuid::Uuid = client_id
        .parse()
        .map_err(|_| server_err("Invalid client_id"))?;
    let from: chrono::NaiveDate = period_from
        .parse()
        .map_err(|_| server_err("Invalid period_from date"))?;
    let to: chrono::NaiveDate = period_to
        .parse()
        .map_err(|_| server_err("Invalid period_to date"))?;

    // Verify client belongs to this org and get its currency.
    let client = sqlx::query_as!(
        Client,
        r#"SELECT id, org_id, name, currency, address, tax_id, active,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM clients WHERE id = $1 AND org_id = $2"#,
        client_id,
        manager.org_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Client not found".into(),
        code: NOT_FOUND,
        details: None,
    })?;

    // Fetch billable, un-invoiced entries for this client in the period,
    // with rate candidates from all cascade levels.
    struct EntryWithRates {
        entry_id: uuid::Uuid,
        minutes: i32,
        project_name: String,
        task_name: String,
        notes: Option<String>,
        spent_date: chrono::NaiveDate,
        task_rate_cents: Option<i64>,
        assignment_rate_cents: Option<i64>,
        user_rate_cents: Option<i64>,
    }

    let entries = sqlx::query_as!(
        EntryWithRates,
        r#"SELECT
             te.id as entry_id,
             te.minutes,
             p.name as project_name,
             t.name as task_name,
             te.notes,
             te.spent_date as "spent_date: chrono::NaiveDate",
             pt.rate_cents as task_rate_cents,
             a.rate_cents as assignment_rate_cents,
             u.billable_rate_cents as user_rate_cents
           FROM time_entries te
           JOIN projects p ON p.id = te.project_id
           JOIN tasks t ON t.id = te.task_id
           LEFT JOIN project_tasks pt ON pt.project_id = te.project_id AND pt.task_id = te.task_id
           LEFT JOIN assignments a ON a.project_id = te.project_id AND a.user_id = te.user_id
           JOIN users u ON u.id = te.user_id
           WHERE te.org_id = $1
             AND p.client_id = $2
             AND te.billable = true
             AND te.invoice_id IS NULL
             AND te.state = 'open'
             AND te.spent_date >= $3
             AND te.spent_date <= $4
           ORDER BY te.spent_date, te.id"#,
        manager.org_id,
        client_id,
        from as chrono::NaiveDate,
        to as chrono::NaiveDate,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    if entries.is_empty() {
        return Err(ServerFnError::ServerError {
            message: "No billable, un-invoiced time found for this client and period.".into(),
            code: NOT_FOUND,
            details: None,
        });
    }

    // Generate invoice number: INV-YYYYMM-NNN
    let now = chrono::Utc::now();
    let year_month = now.format("%Y%m").to_string();
    let count: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!: i64" FROM invoices
           WHERE org_id = $1 AND number LIKE $2"#,
        manager.org_id,
        format!("INV-{year_month}-%"),
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;
    let invoice_number = format!("INV-{year_month}-{:03}", count + 1);

    let invoice_id = uuid::Uuid::now_v7();
    let issued_on = now.date_naive();
    // Default due date: 30 days from issue
    let due_on = issued_on + chrono::Duration::days(30);

    // Build line items and compute total.
    let mut lines = Vec::with_capacity(entries.len());
    let mut total_cents: i64 = 0;

    for e in &entries {
        let rate = horae_core::invoice::resolve_rate(
            e.task_rate_cents,
            e.assignment_rate_cents,
            e.user_rate_cents,
        )
        .unwrap_or(0);

        let amount = horae_core::invoice::line_amount_cents(rate, e.minutes);
        total_cents += amount;

        let description = if let Some(notes) = &e.notes {
            format!(
                "{} — {} ({}): {}",
                e.spent_date, e.project_name, e.task_name, notes
            )
        } else {
            format!("{} — {} ({})", e.spent_date, e.project_name, e.task_name)
        };

        lines.push(InvoiceLine {
            id: uuid::Uuid::now_v7(),
            invoice_id,
            time_entry_id: e.entry_id,
            description,
            minutes: e.minutes,
            rate_cents: rate,
            amount_cents: amount,
        });
    }

    // Insert invoice.
    sqlx::query!(
        r#"INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents)
           VALUES ($1, $2, $3, $4, 'draft', $5, $6, $7, $8)"#,
        invoice_id,
        manager.org_id,
        client_id,
        invoice_number,
        issued_on as chrono::NaiveDate,
        due_on as chrono::NaiveDate,
        client.currency.trim(),
        total_cents,
    )
    .execute(&state.db)
    .await
    .map_err(server_err)?;

    // Insert line items.
    for line in &lines {
        sqlx::query!(
            r#"INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents)
               VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
            line.id,
            line.invoice_id,
            line.time_entry_id,
            line.description,
            line.minutes,
            line.rate_cents,
            line.amount_cents,
        )
        .execute(&state.db)
        .await
        .map_err(server_err)?;
    }

    // Mark entries as invoiced.
    let entry_ids: Vec<uuid::Uuid> = entries.iter().map(|e| e.entry_id).collect();
    sqlx::query!(
        r#"UPDATE time_entries
           SET invoice_id = $1,
               state = 'invoiced',
               updated_at = now()
           WHERE id = ANY($2)"#,
        invoice_id,
        &entry_ids,
    )
    .execute(&state.db)
    .await
    .map_err(server_err)?;

    let invoice = Invoice {
        id: invoice_id,
        org_id: manager.org_id,
        client_id,
        number: invoice_number.clone(),
        status: InvoiceStatus::Draft,
        issued_on,
        due_on,
        currency: client.currency.trim().to_string(),
        total_cents,
        notes: None,
        created_at: now,
    };

    // Dispatch invoice_created event (FR-019).
    let state = crate::state::global_state().await;
    state
        .plugins
        .dispatch(crate::plugin::AppEvent::InvoiceCreated {
            occurred_at: chrono::Utc::now(),
            org_id: manager.org_id,
            invoice: crate::plugin::event::InvoicePayload {
                id: invoice_id,
                client_id,
                invoice_number,
                status: "draft".into(),
                issue_date: issued_on,
                due_date: due_on,
                currency: invoice.currency.clone(),
                total_cents,
            },
        });

    Ok(InvoiceWithLines { invoice, lines })
}

#[server]
pub async fn update_invoice_status(
    invoice_id: String,
    new_status: String,
) -> Result<Invoice, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let id: uuid::Uuid = invoice_id
        .parse()
        .map_err(|_| server_err("Invalid invoice_id"))?;
    let target: InvoiceStatus = new_status
        .parse()
        .map_err(|_| server_err("Invalid status"))?;

    let current_status: InvoiceStatus = sqlx::query_scalar!(
        r#"SELECT status as "status: InvoiceStatus"
           FROM invoices WHERE id = $1 AND org_id = $2"#,
        id,
        manager.org_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Invoice not found".into(),
        code: NOT_FOUND,
        details: None,
    })?;

    // Enforce state machine: draft->sent, sent->paid, draft|sent->void
    let valid = matches!(
        (current_status, target),
        (InvoiceStatus::Draft, InvoiceStatus::Sent)
            | (InvoiceStatus::Sent, InvoiceStatus::Paid)
            | (InvoiceStatus::Draft, InvoiceStatus::Void)
            | (InvoiceStatus::Sent, InvoiceStatus::Void)
    );
    if !valid {
        return Err(ServerFnError::ServerError {
            message: format!(
                "Cannot transition invoice from {} to {}",
                current_status, target
            ),
            code: CONFLICT,
            details: None,
        });
    }

    // On void: restore entries to open, un-invoiced state.
    if target == InvoiceStatus::Void {
        sqlx::query!(
            r#"UPDATE time_entries
               SET invoice_id = NULL, state = 'open', updated_at = now()
               WHERE invoice_id = $1"#,
            id,
        )
        .execute(&state.db)
        .await
        .map_err(server_err)?;
    }

    let invoice = sqlx::query_as!(
        Invoice,
        r#"UPDATE invoices SET status = $3
           WHERE id = $1 AND org_id = $2
           RETURNING id, org_id, client_id, number,
                     status as "status: InvoiceStatus",
                     issued_on as "issued_on: chrono::NaiveDate",
                     due_on as "due_on: chrono::NaiveDate",
                     currency, total_cents, notes,
                     created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        manager.org_id,
        target as InvoiceStatus,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    // Dispatch invoice_sent event when transitioning to Sent (FR-019).
    if target == InvoiceStatus::Sent {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::InvoiceSent {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                invoice: crate::plugin::event::InvoicePayload {
                    id: invoice.id,
                    client_id: invoice.client_id,
                    invoice_number: invoice.number.clone(),
                    status: "sent".into(),
                    issue_date: invoice.issued_on,
                    due_date: invoice.due_on,
                    currency: invoice.currency.clone(),
                    total_cents: invoice.total_cents,
                },
            });
    }

    if target == InvoiceStatus::Paid {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::InvoicePaid {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                invoice: invoice_payload(&invoice),
            });
    } else if target == InvoiceStatus::Void {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::InvoiceVoided {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                invoice: invoice_payload(&invoice),
            });
    }

    Ok(invoice)
}
