use dioxus::prelude::*;

use crate::models::{Client, Invoice, Project, Task, TimeEntry, User};

fn server_err(msg: impl std::fmt::Display) -> ServerFnError {
    ServerFnError::ServerError {
        message: msg.to_string(),
        code: 500,
        details: None,
    }
}

// ── Auth ─────────────────────────────────────────────────────────────────────

#[server]
pub async fn login(email: String, password: String) -> Result<(), ServerFnError> {
    let _state = crate::state::global_state().await;
    // TODO: query user, verify argon2 password, set session
    let _ = (email, password);
    Ok(())
}

#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    // TODO: clear session
    Ok(())
}

// ── Time Entries ─────────────────────────────────────────────────────────────

#[server]
pub async fn list_time_entries(
    user_id: Option<String>,
    project_id: Option<String>,
    date_from: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<TimeEntry>, ServerFnError> {
    let state = crate::state::global_state().await;
    let limit = limit.unwrap_or(50);

    let entries = sqlx::query_as::<_, TimeEntry>(
        "SELECT id, user_id, project_id, task_id, started_at, ended_at,
                duration_seconds, notes, is_billable, invoice_id, created_at, updated_at
         FROM time_entries
         ORDER BY started_at DESC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    let _ = (user_id, project_id, date_from);
    Ok(entries)
}

#[server]
pub async fn start_timer(project_id: String, task_id: Option<String>) -> Result<TimeEntry, ServerFnError> {
    use chrono::Utc;
    use uuid::Uuid;

    let _state = crate::state::global_state().await;
    let _ = task_id;
    let project_uuid = project_id.parse::<Uuid>().map_err(|e| server_err(e))?;
    let now = Utc::now();
    Ok(TimeEntry {
        id: Uuid::now_v7(),
        user_id: Uuid::now_v7(),
        project_id: project_uuid,
        task_id: None,
        started_at: now,
        ended_at: None,
        duration_seconds: 0,
        notes: None,
        is_billable: true,
        invoice_id: None,
        created_at: now,
        updated_at: now,
    })
}

#[server]
pub async fn stop_timer(entry_id: String) -> Result<TimeEntry, ServerFnError> {
    use chrono::Utc;
    use uuid::Uuid;

    let _state = crate::state::global_state().await;
    let entry_uuid = entry_id.parse::<Uuid>().map_err(|e| server_err(e))?;
    let now = Utc::now();
    Ok(TimeEntry {
        id: entry_uuid,
        user_id: Uuid::now_v7(),
        project_id: Uuid::now_v7(),
        task_id: None,
        started_at: now,
        ended_at: Some(now),
        duration_seconds: 0,
        notes: None,
        is_billable: true,
        invoice_id: None,
        created_at: now,
        updated_at: now,
    })
}

// ── Clients ──────────────────────────────────────────────────────────────────

#[server]
pub async fn list_clients() -> Result<Vec<Client>, ServerFnError> {
    let state = crate::state::global_state().await;

    let clients = sqlx::query_as::<_, Client>(
        "SELECT id, name, email, currency, created_by, created_at, updated_at
         FROM clients
         ORDER BY name ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    Ok(clients)
}

// ── Projects ─────────────────────────────────────────────────────────────────

#[server]
pub async fn list_projects(
    client_id: Option<String>,
    active_only: Option<bool>,
) -> Result<Vec<Project>, ServerFnError> {
    let state = crate::state::global_state().await;

    let projects = sqlx::query_as::<_, Project>(
        "SELECT id, client_id, name, code, budget_hours, billing_method,
                hourly_rate, is_active, created_at, updated_at
         FROM projects
         WHERE (? IS NULL OR is_active = ?)
         ORDER BY name ASC",
    )
    .bind(active_only.map(|b| if b { 1i64 } else { 0i64 }))
    .bind(active_only.map(|b| if b { 1i64 } else { 0i64 }))
    .fetch_all(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    let _ = client_id;
    Ok(projects)
}

// ── Tasks ────────────────────────────────────────────────────────────────────

#[server]
pub async fn list_tasks(project_id: Option<String>) -> Result<Vec<Task>, ServerFnError> {
    let state = crate::state::global_state().await;

    let tasks = sqlx::query_as::<_, crate::models::Task>(
        "SELECT id, project_id, name, hourly_rate, is_billable, created_at, updated_at
         FROM tasks
         WHERE (? IS NULL OR project_id = ?)
         ORDER BY name ASC",
    )
    .bind(&project_id)
    .bind(&project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    Ok(tasks)
}

// ── Invoices ─────────────────────────────────────────────────────────────────

#[server]
pub async fn list_invoices(status: Option<String>) -> Result<Vec<Invoice>, ServerFnError> {
    let state = crate::state::global_state().await;

    let invoices = sqlx::query_as::<_, Invoice>(
        "SELECT id, client_id, invoice_number, status, issued_date, due_date,
                total_amount, created_at, updated_at
         FROM invoices
         WHERE (? IS NULL OR status = ?)
         ORDER BY issued_date DESC",
    )
    .bind(&status)
    .bind(&status)
    .fetch_all(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    Ok(invoices)
}

// ── Users ─────────────────────────────────────────────────────────────────────

#[server]
pub async fn list_users() -> Result<Vec<User>, ServerFnError> {
    let state = crate::state::global_state().await;

    let users = sqlx::query_as::<_, User>(
        "SELECT id, email, display_name, password_hash, role, is_active, created_at, updated_at
         FROM users
         ORDER BY display_name ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    Ok(users)
}
