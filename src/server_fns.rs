use dioxus::prelude::*;

use crate::models::{Client, Invoice, Project, Task, TimeEntry, User};

/// Extract the session user UUID from the request context.
/// Returns `Err(401)` if the session has no user (not logged in).
#[cfg(feature = "server")]
async fn session_user_id() -> Result<uuid::Uuid, ServerFnError> {
    use tower_sessions::Session;

    // `FullstackContext::extract` pulls the Session from the Axum request extensions
    // injected by the SessionManagerLayer wrapping the server.
    // Session implements FromRequestParts for any S, so M is inferred as the
    // axum via::Parts marker type.
    let session: Session =
        dioxus_fullstack::FullstackContext::extract::<Session, _>().await?;

    crate::auth::session::get_session_user_id(&session)
        .await
        .ok_or_else(|| ServerFnError::ServerError {
            message: "Not authenticated — please sign in.".into(),
            code: 401,
            details: None,
        })
}

fn server_err(msg: impl std::fmt::Display) -> ServerFnError {
    ServerFnError::ServerError {
        message: msg.to_string(),
        code: 500,
        details: None,
    }
}

// ── Auth ─────────────────────────────────────────────────────────────────────

/// Stub login — the real auth flows go through Axum routes at `/auth/login`.
#[server]
pub async fn login(email: String, password: String) -> Result<(), ServerFnError> {
    let _ = (email, password);
    Err(ServerFnError::ServerError {
        message: "Direct login removed; navigate to /auth/login.".into(),
        code: 401,
        details: None,
    })
}

/// Destroy the current session (logout).
#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    use tower_sessions::Session;

    let session: Session =
        dioxus_fullstack::FullstackContext::extract::<Session, _>().await?;

    crate::auth::session::clear_session(&session)
        .await
        .map_err(|e| server_err(e))
}

/// Return the currently authenticated user, or 401 if not logged in.
#[server]
pub async fn get_me() -> Result<User, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;

    sqlx::query_as::<_, User>(
        "SELECT id, org_id, email, name, oidc_subject, org_role::text AS org_role,
                cost_rate_cents, billable_rate_cents, active, created_at
         FROM users
         WHERE id = $1 AND active = true",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| server_err(e))?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "User not found".into(),
        code: 404,
        details: None,
    })
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
    let _ = (user_id, project_id, date_from);

    let entries = sqlx::query_as::<_, TimeEntry>(
        "SELECT id, org_id, user_id, project_id, task_id, spent_date,
                minutes, rounded_minutes, notes, billable, is_running, started_at,
                state::text AS state, invoice_id, created_at, updated_at
         FROM time_entries
         ORDER BY spent_date DESC, created_at DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    Ok(entries)
}

/// Stub timer functions — rewritten in M5 with real DB logic.
#[server]
pub async fn start_timer(
    project_id: String,
    task_id: Option<String>,
) -> Result<TimeEntry, ServerFnError> {
    let _ = (project_id, task_id);
    Err(ServerFnError::ServerError {
        message: "Timer not yet implemented (M5).".into(),
        code: 501,
        details: None,
    })
}

#[server]
pub async fn stop_timer(entry_id: String) -> Result<TimeEntry, ServerFnError> {
    let _ = entry_id;
    Err(ServerFnError::ServerError {
        message: "Timer not yet implemented (M5).".into(),
        code: 501,
        details: None,
    })
}

#[server]
pub async fn get_current_timer() -> Result<Option<TimeEntry>, ServerFnError> {
    let state = crate::state::global_state().await;

    // TODO: derive user_id from session in M3; for now return nothing
    let entry = sqlx::query_as::<_, TimeEntry>(
        "SELECT id, org_id, user_id, project_id, task_id, spent_date,
                minutes, rounded_minutes, notes, billable, is_running, started_at,
                state::text AS state, invoice_id, created_at, updated_at
         FROM time_entries
         WHERE is_running = true
         LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    Ok(entry)
}

// ── Clients ──────────────────────────────────────────────────────────────────

#[server]
pub async fn list_clients() -> Result<Vec<Client>, ServerFnError> {
    let state = crate::state::global_state().await;

    let clients = sqlx::query_as::<_, Client>(
        "SELECT id, org_id, name, currency, address, tax_id, active, created_at
         FROM clients
         WHERE active = true
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
    let _ = client_id;
    let active = active_only.unwrap_or(true);

    let projects = sqlx::query_as::<_, Project>(
        "SELECT id, org_id, client_id, code, name,
                project_type::text AS project_type, currency,
                starts_on, ends_on,
                budget_kind::text AS budget_kind,
                budget_amount_cents, budget_minutes, active, created_at
         FROM projects
         WHERE ($1 IS NULL OR active = $1)
         ORDER BY name ASC",
    )
    .bind(if active { Some(true) } else { None::<bool> })
    .fetch_all(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    Ok(projects)
}

// ── Tasks ────────────────────────────────────────────────────────────────────

/// Lists all org-level tasks.
///
/// The `project_id` parameter is kept for backwards compatibility with pages that
/// pass it; in the new schema tasks are org-level (filtering by project is done
/// via `project_tasks` in M4).
#[server]
pub async fn list_tasks(project_id: Option<String>) -> Result<Vec<Task>, ServerFnError> {
    let state = crate::state::global_state().await;
    let _ = project_id;

    let tasks = sqlx::query_as::<_, Task>(
        "SELECT id, org_id, name, billable_default, default_rate_cents, active
         FROM tasks
         WHERE active = true
         ORDER BY name ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    Ok(tasks)
}

// ── Invoices (Phase 4 — stub) ─────────────────────────────────────────────────

#[server]
pub async fn list_invoices(status: Option<String>) -> Result<Vec<Invoice>, ServerFnError> {
    // Invoices are Phase 4; the table doesn't exist yet.
    let _ = status;
    Ok(vec![])
}

// ── Users ─────────────────────────────────────────────────────────────────────

#[server]
pub async fn list_users() -> Result<Vec<User>, ServerFnError> {
    let state = crate::state::global_state().await;

    let users = sqlx::query_as::<_, User>(
        "SELECT id, org_id, email, name, oidc_subject,
                org_role::text AS org_role,
                cost_rate_cents, billable_rate_cents, active, created_at
         FROM users
         WHERE active = true
         ORDER BY name ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    Ok(users)
}
