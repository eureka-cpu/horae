use dioxus::prelude::*;
#[cfg(feature = "server")]
use horae_core::types::{
    BudgetKind, EntryState, InvoiceStatus, OrgRole, ProjectRole, ProjectType, RoundDir,
};

#[cfg(feature = "server")]
use crate::models::InvoiceLine;
use crate::models::{
    Approval, Assignment, Client, DetailedReportRow, Invoice, InvoiceWithLines, OrgBranding,
    Project, ReportRow, Task, TimeEntry, User,
};

// HTTP status codes for `ServerFnError::ServerError { code, .. }` — named so
// error paths read at a glance (see AGENTS.md conventions). Server-only: on the
// web target `#[server]` bodies are replaced by client stubs that never build
// these errors.
#[cfg(feature = "server")]
mod status {
    pub const UNAUTHORIZED: u16 = 401;
    pub const FORBIDDEN: u16 = 403;
    pub const NOT_FOUND: u16 = 404;
    pub const CONFLICT: u16 = 409;
    pub const INTERNAL_ERROR: u16 = 500;
}
#[cfg(feature = "server")]
use status::*;

/// Extract the session user UUID from the request context.
/// Returns `Err(401)` if the session has no user (not logged in).
#[cfg(feature = "server")]
async fn session_user_id() -> Result<uuid::Uuid, ServerFnError> {
    use tower_sessions::Session;

    // `FullstackContext::extract` pulls the Session from the Axum request extensions
    // injected by the SessionManagerLayer wrapping the server.
    // Session implements FromRequestParts for any S, so M is inferred as the
    // axum via::Parts marker type.
    let session: Session = dioxus_fullstack::FullstackContext::extract::<Session, _>().await?;

    crate::auth::session::get_session_user_id(&session)
        .await
        .ok_or_else(|| ServerFnError::ServerError {
            message: "Not authenticated — please sign in.".into(),
            code: UNAUTHORIZED,
            details: None,
        })
}

#[cfg(feature = "server")]
fn server_err(msg: impl std::fmt::Display) -> ServerFnError {
    ServerFnError::ServerError {
        message: msg.to_string(),
        code: INTERNAL_ERROR,
        details: None,
    }
}

#[cfg(feature = "server")]
async fn require_admin() -> Result<crate::models::User, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let user = sqlx::query_as!(
        crate::models::User,
        r#"SELECT id, org_id, email, name, oidc_subject,
                org_role as "org_role: OrgRole",
                cost_rate_cents, billable_rate_cents, active,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM users WHERE id = $1 AND active = true"#,
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "User not found".into(),
        code: NOT_FOUND,
        details: None,
    })?;

    if !user.is_admin() {
        return Err(ServerFnError::ServerError {
            message: "Admin access required".into(),
            code: FORBIDDEN,
            details: None,
        });
    }
    Ok(user)
}

#[cfg(feature = "server")]
async fn require_manager() -> Result<crate::models::User, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let user = sqlx::query_as!(
        crate::models::User,
        r#"SELECT id, org_id, email, name, oidc_subject,
                org_role as "org_role: OrgRole",
                cost_rate_cents, billable_rate_cents, active,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM users WHERE id = $1 AND active = true"#,
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "User not found".into(),
        code: NOT_FOUND,
        details: None,
    })?;

    if !user.is_manager_or_above() {
        return Err(ServerFnError::ServerError {
            message: "Manager access required".into(),
            code: FORBIDDEN,
            details: None,
        });
    }
    Ok(user)
}

// ── Plugin event dispatch helpers ────────────────────────────────────────────
// Fire-and-forget: spawn dispatch so the core action is never blocked (FR-021).

#[cfg(feature = "server")]
async fn dispatch_time_entry_event(entry: &crate::models::TimeEntry, event_name: &str) {
    let state = crate::state::global_state().await;
    let event = match event_name {
        "time_entry_created" => crate::plugin::AppEvent::TimeEntryCreated {
            occurred_at: chrono::Utc::now(),
            org_id: entry.org_id,
            time_entry: crate::plugin::event::TimeEntryPayload {
                id: entry.id,
                user_id: entry.user_id,
                project_id: entry.project_id,
                task_id: entry.task_id,
                spent_date: entry.spent_date,
                minutes: entry.minutes,
                billable: entry.billable,
                is_running: entry.is_running,
                notes: entry.notes.clone(),
                started_at: entry.started_at,
            },
        },
        _ => crate::plugin::AppEvent::TimeEntryStopped {
            occurred_at: chrono::Utc::now(),
            org_id: entry.org_id,
            time_entry: crate::plugin::event::TimeEntryPayload {
                id: entry.id,
                user_id: entry.user_id,
                project_id: entry.project_id,
                task_id: entry.task_id,
                spent_date: entry.spent_date,
                minutes: entry.minutes,
                billable: entry.billable,
                is_running: entry.is_running,
                notes: entry.notes.clone(),
                started_at: entry.started_at,
            },
        },
    };
    state.plugins.dispatch(event);
}

// ── Auth ─────────────────────────────────────────────────────────────────────

/// Stub login — the real auth flows go through Axum routes at `/auth/login`.
#[server]
pub async fn login(email: String, password: String) -> Result<(), ServerFnError> {
    let _ = (email, password);
    Err(ServerFnError::ServerError {
        message: "Direct login removed; navigate to /auth/login.".into(),
        code: UNAUTHORIZED,
        details: None,
    })
}

/// Destroy the current session (logout).
#[server]
pub async fn logout() -> Result<(), ServerFnError> {
    use tower_sessions::Session;

    let session: Session = dioxus_fullstack::FullstackContext::extract::<Session, _>().await?;

    crate::auth::session::clear_session(&session)
        .await
        .map_err(server_err)
}

/// Return the currently authenticated user, or 401 if not logged in.
#[server]
pub async fn get_me() -> Result<User, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;

    sqlx::query_as!(
        User,
        r#"SELECT id, org_id, email, name, oidc_subject,
                org_role as "org_role: OrgRole",
                cost_rate_cents, billable_rate_cents, active,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM users
         WHERE id = $1 AND active = true"#,
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "User not found".into(),
        code: NOT_FOUND,
        details: None,
    })
}

// ── Time Entries ─────────────────────────────────────────────────────────────

#[server]
pub async fn list_time_entries(
    _user_id: Option<String>,
    project_id: Option<String>,
    date_from: Option<String>,
    date_to: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<TimeEntry>, ServerFnError> {
    let session_uid = session_user_id().await?;
    let state = crate::state::global_state().await;
    let limit = limit.unwrap_or(50);

    let project_filter: Option<uuid::Uuid> = match project_id {
        Some(ref s) => Some(s.parse().map_err(|_| server_err("Invalid project_id"))?),
        None => None,
    };
    let date_filter: Option<chrono::NaiveDate> = match date_from {
        Some(ref s) => Some(
            s.parse()
                .map_err(|_| server_err("Invalid date_from (use YYYY-MM-DD)"))?,
        ),
        None => None,
    };
    let date_to_filter: Option<chrono::NaiveDate> = match date_to {
        Some(ref s) => Some(
            s.parse()
                .map_err(|_| server_err("Invalid date_to (use YYYY-MM-DD)"))?,
        ),
        None => None,
    };

    let entries = sqlx::query_as!(
        TimeEntry,
        r#"SELECT id, org_id, user_id, project_id, task_id,
                spent_date as "spent_date: chrono::NaiveDate",
                minutes, rounded_minutes, notes, billable, is_running,
                started_at as "started_at: chrono::DateTime<chrono::Utc>",
                state as "state: EntryState", invoice_id,
                created_at as "created_at: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
         FROM time_entries
         WHERE user_id = $1
           AND ($2::uuid IS NULL OR project_id = $2)
           AND ($3::date IS NULL OR spent_date >= $3)
           AND ($4::date IS NULL OR spent_date <= $4)
         ORDER BY spent_date DESC, created_at DESC
         LIMIT $5"#,
        session_uid,
        project_filter,
        date_filter as Option<chrono::NaiveDate>,
        date_to_filter as Option<chrono::NaiveDate>,
        limit,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(entries)
}

/// Start a timer for the given project and task. Only one timer may run at a time
/// per user (enforced both here and via a DB partial unique index).
#[server]
pub async fn start_timer(
    project_id: String,
    task_id: String,
    notes: Option<String>,
) -> Result<TimeEntry, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let project_id: uuid::Uuid = project_id
        .parse()
        .map_err(|_| server_err("Invalid project_id"))?;
    let task_id: uuid::Uuid = task_id.parse().map_err(|_| server_err("Invalid task_id"))?;

    // Get user's org_id
    let user = sqlx::query_as!(
        User,
        r#"SELECT id, org_id, email, name, oidc_subject,
                org_role as "org_role: OrgRole",
                cost_rate_cents, billable_rate_cents, active,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM users WHERE id = $1"#,
        user_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    // Check no timer already running
    let existing = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM time_entries WHERE user_id = $1 AND is_running = true)",
        user_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?
    .unwrap_or(false);

    if existing {
        return Err(ServerFnError::ServerError {
            message: "A timer is already running. Stop it first.".into(),
            code: CONFLICT,
            details: None,
        });
    }

    let id = uuid::Uuid::now_v7();
    let today = chrono::Utc::now().date_naive();

    let entry = sqlx::query_as!(
        TimeEntry,
        r#"INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, notes, billable, is_running, started_at, state)
         VALUES ($1, $2, $3, $4, $5, $6, 0, $7, true, true, now(), $8)
         RETURNING id, org_id, user_id, project_id, task_id,
                   spent_date as "spent_date: chrono::NaiveDate",
                   minutes, rounded_minutes, notes, billable, is_running,
                   started_at as "started_at: chrono::DateTime<chrono::Utc>",
                   state as "state: EntryState", invoice_id,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        id,
        user.org_id,
        user_id,
        project_id,
        task_id,
        today as chrono::NaiveDate,
        notes.as_deref(),
        EntryState::Open as EntryState,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    dispatch_time_entry_event(&entry, "time_entry_created").await;
    Ok(entry)
}

/// Stop a running timer and record elapsed minutes.
#[server]
pub async fn stop_timer(entry_id: String) -> Result<TimeEntry, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let entry_id: uuid::Uuid = entry_id
        .parse()
        .map_err(|_| server_err("Invalid entry_id"))?;

    // Read the running entry's start time, then compute the exact elapsed
    // minutes in `horae-core` (floored to the minute, no artificial 1-minute
    // minimum) so tracked totals stay exact (FR-003/FR-023).
    let started_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar!(
        r#"SELECT started_at as "started_at: chrono::DateTime<chrono::Utc>"
               FROM time_entries
               WHERE id = $1 AND user_id = $2 AND is_running = true"#,
        entry_id,
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .flatten()
    .ok_or_else(|| ServerFnError::ServerError {
        message: "No running timer found for this entry".into(),
        code: axum::http::StatusCode::NOT_FOUND.as_u16(),
        details: None,
    })?;

    let minutes = horae_core::duration::minutes_between(started_at, chrono::Utc::now()) as i32;

    let entry = sqlx::query_as!(
        TimeEntry,
        r#"UPDATE time_entries
         SET is_running = false,
             minutes = $3,
             started_at = NULL,
             updated_at = now()
         WHERE id = $1 AND user_id = $2 AND is_running = true
         RETURNING id, org_id, user_id, project_id, task_id,
                   spent_date as "spent_date: chrono::NaiveDate",
                   minutes, rounded_minutes, notes, billable, is_running,
                   started_at as "started_at: chrono::DateTime<chrono::Utc>",
                   state as "state: EntryState", invoice_id,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        entry_id,
        user_id,
        minutes,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "No running timer found for this entry".into(),
        code: NOT_FOUND,
        details: None,
    })?;

    dispatch_time_entry_event(&entry, "time_entry_stopped").await;
    Ok(entry)
}

/// Return the currently running timer for the authenticated user, if any.
#[server]
pub async fn get_current_timer() -> Result<Option<TimeEntry>, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;

    let entry = sqlx::query_as!(
        TimeEntry,
        r#"SELECT id, org_id, user_id, project_id, task_id,
                spent_date as "spent_date: chrono::NaiveDate",
                minutes, rounded_minutes, notes, billable, is_running,
                started_at as "started_at: chrono::DateTime<chrono::Utc>",
                state as "state: EntryState", invoice_id,
                created_at as "created_at: chrono::DateTime<chrono::Utc>",
                updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
         FROM time_entries
         WHERE user_id = $1 AND is_running = true
         LIMIT 1"#,
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?;

    Ok(entry)
}

/// Create a manual (non-timer) time entry.
#[server]
pub async fn create_time_entry(
    project_id: String,
    task_id: String,
    spent_date: String,
    minutes: i32,
    notes: Option<String>,
    billable: bool,
) -> Result<TimeEntry, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let project_id: uuid::Uuid = project_id
        .parse()
        .map_err(|_| server_err("Invalid project_id"))?;
    let task_id: uuid::Uuid = task_id.parse().map_err(|_| server_err("Invalid task_id"))?;
    let spent_date: chrono::NaiveDate = spent_date
        .parse()
        .map_err(|_| server_err("Invalid date (use YYYY-MM-DD)"))?;

    let row = sqlx::query!(
        r#"SELECT org_id, org_role as "org_role: OrgRole" FROM users WHERE id = $1"#,
        user_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    // Check assignment (skip for admins)
    if row.org_role != OrgRole::Admin {
        let assigned = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM assignments WHERE project_id = $1 AND user_id = $2)",
            project_id,
            user_id,
        )
        .fetch_one(&state.db)
        .await
        .map_err(server_err)?
        .unwrap_or(false);

        if !assigned {
            return Err(ServerFnError::ServerError {
                message: "You are not assigned to this project".into(),
                code: FORBIDDEN,
                details: None,
            });
        }
    }

    let id = uuid::Uuid::now_v7();

    let entry = sqlx::query_as!(
        TimeEntry,
        r#"INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, notes, billable, is_running, state)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, false, $10)
         RETURNING id, org_id, user_id, project_id, task_id,
                   spent_date as "spent_date: chrono::NaiveDate",
                   minutes, rounded_minutes, notes, billable, is_running,
                   started_at as "started_at: chrono::DateTime<chrono::Utc>",
                   state as "state: EntryState", invoice_id,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        id,
        row.org_id,
        user_id,
        project_id,
        task_id,
        spent_date as chrono::NaiveDate,
        minutes,
        notes.as_deref(),
        billable,
        EntryState::Open as EntryState,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    dispatch_time_entry_event(&entry, "time_entry_created").await;
    Ok(entry)
}

/// Update a time entry. Only allowed while the entry state is 'open'.
#[server]
pub async fn update_time_entry(
    entry_id: String,
    minutes: i32,
    notes: Option<String>,
    billable: bool,
) -> Result<TimeEntry, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let entry_id: uuid::Uuid = entry_id
        .parse()
        .map_err(|_| server_err("Invalid entry_id"))?;

    sqlx::query_as!(
        TimeEntry,
        r#"UPDATE time_entries
         SET minutes = $3, notes = $4, billable = $5, updated_at = now()
         WHERE id = $1 AND user_id = $2 AND state = $6
         RETURNING id, org_id, user_id, project_id, task_id,
                   spent_date as "spent_date: chrono::NaiveDate",
                   minutes, rounded_minutes, notes, billable, is_running,
                   started_at as "started_at: chrono::DateTime<chrono::Utc>",
                   state as "state: EntryState", invoice_id,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>",
                   updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        entry_id,
        user_id,
        minutes,
        notes.as_deref(),
        billable,
        EntryState::Open as EntryState,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Entry not found or is locked (not in 'open' state)".into(),
        code: CONFLICT,
        details: None,
    })
}

/// Delete a time entry. Only allowed while the entry state is 'open'.
#[server]
pub async fn delete_time_entry(entry_id: String) -> Result<(), ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let entry_id: uuid::Uuid = entry_id
        .parse()
        .map_err(|_| server_err("Invalid entry_id"))?;

    let result = sqlx::query!(
        "DELETE FROM time_entries WHERE id = $1 AND user_id = $2 AND state = $3",
        entry_id,
        user_id,
        EntryState::Open as EntryState,
    )
    .execute(&state.db)
    .await
    .map_err(server_err)?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::ServerError {
            message: "Entry not found or is locked (not in 'open' state)".into(),
            code: CONFLICT,
            details: None,
        });
    }

    Ok(())
}

// ── Clients ──────────────────────────────────────────────────────────────────

/// Lists clients. With `include_inactive = false` only active clients are
/// returned (the set shown in new-entry pickers); pass `true` for the management
/// view that also needs to reactivate deactivated clients.
#[server]
pub async fn list_clients(include_inactive: bool) -> Result<Vec<Client>, ServerFnError> {
    let state = crate::state::global_state().await;

    let clients = sqlx::query_as!(
        Client,
        r#"SELECT id, org_id, name, currency, address, tax_id, active,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM clients
         WHERE ($1::bool OR active = true)
         ORDER BY name ASC"#,
        include_inactive,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(clients)
}

#[server]
pub async fn create_client(
    name: String,
    currency: String,
    address: Option<String>,
    tax_id: Option<String>,
) -> Result<Client, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    sqlx::query_as!(
        Client,
        r#"INSERT INTO clients (id, org_id, name, currency, address, tax_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, org_id, name, currency, address, tax_id, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        manager.org_id,
        name,
        currency,
        address.as_deref(),
        tax_id.as_deref(),
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)
}

#[server]
pub async fn update_client(
    client_id: String,
    name: String,
    currency: String,
    address: Option<String>,
    tax_id: Option<String>,
) -> Result<Client, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let client_id: uuid::Uuid = client_id
        .parse()
        .map_err(|_| server_err("Invalid client_id"))?;
    sqlx::query_as::<_, Client>(
        "UPDATE clients SET name = $3, currency = $4, address = $5, tax_id = $6
         WHERE id = $1 AND org_id = $2
         RETURNING id, org_id, name, currency, address, tax_id, active, created_at",
    )
    .bind(client_id)
    .bind(manager.org_id)
    .bind(&name)
    .bind(&currency)
    .bind(&address)
    .bind(&tax_id)
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Client not found".into(),
        code: NOT_FOUND,
        details: None,
    })
}

/// Activate or deactivate a client. Deactivated clients are hidden from
/// new-entry pickers but remain linked to existing projects and entries (FR-011).
#[server]
pub async fn set_client_active(client_id: String, active: bool) -> Result<Client, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let client_id: uuid::Uuid = client_id
        .parse()
        .map_err(|_| server_err("Invalid client_id"))?;
    sqlx::query_as::<_, Client>(
        "UPDATE clients SET active = $3
         WHERE id = $1 AND org_id = $2
         RETURNING id, org_id, name, currency, address, tax_id, active, created_at",
    )
    .bind(client_id)
    .bind(manager.org_id)
    .bind(active)
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Client not found".into(),
        code: NOT_FOUND,
        details: None,
    })
}

// ── Projects ─────────────────────────────────────────────────────────────────

#[server]
pub async fn list_projects(
    client_id: Option<String>,
    include_inactive: bool,
) -> Result<Vec<Project>, ServerFnError> {
    let state = crate::state::global_state().await;
    let _ = client_id;

    let projects = sqlx::query_as!(
        Project,
        r#"SELECT id, org_id, client_id, code, name,
                project_type as "project_type: ProjectType", currency,
                starts_on as "starts_on: chrono::NaiveDate",
                ends_on as "ends_on: chrono::NaiveDate",
                budget_kind as "budget_kind: BudgetKind",
                budget_amount_cents, budget_minutes, active,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM projects
         WHERE ($1::bool OR active = true)
         ORDER BY name ASC"#,
        include_inactive,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(projects)
}

#[server]
pub async fn create_project(
    client_id: String,
    name: String,
    project_type: String,
    currency: String,
    budget_kind: String,
) -> Result<Project, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    let client_id: uuid::Uuid = client_id
        .parse()
        .map_err(|_| server_err("Invalid client_id"))?;
    let pt = project_type
        .parse::<ProjectType>()
        .map_err(|_| server_err("Invalid project_type"))?;
    let bk = budget_kind
        .parse::<BudgetKind>()
        .map_err(|_| server_err("Invalid budget_kind"))?;
    sqlx::query_as!(
        Project,
        r#"INSERT INTO projects (id, org_id, client_id, name, project_type, currency, budget_kind)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, org_id, client_id, code, name,
                   project_type as "project_type: ProjectType", currency,
                   starts_on as "starts_on: chrono::NaiveDate",
                   ends_on as "ends_on: chrono::NaiveDate",
                   budget_kind as "budget_kind: BudgetKind",
                   budget_amount_cents, budget_minutes, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        manager.org_id,
        client_id,
        name,
        pt as ProjectType,
        currency,
        bk as BudgetKind,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)
}

#[server]
pub async fn update_project(
    project_id: String,
    name: String,
    project_type: String,
    currency: String,
    budget_kind: String,
) -> Result<Project, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let project_id: uuid::Uuid = project_id
        .parse()
        .map_err(|_| server_err("Invalid project_id"))?;
    sqlx::query_as::<_, Project>(
        "UPDATE projects
            SET name = $3, project_type = $4, currency = $5, budget_kind = $6
          WHERE id = $1 AND org_id = $2
         RETURNING id, org_id, client_id, code, name,
                   project_type, currency,
                   starts_on, ends_on,
                   budget_kind,
                   budget_amount_cents, budget_minutes, active, created_at",
    )
    .bind(project_id)
    .bind(manager.org_id)
    .bind(&name)
    .bind(
        project_type
            .parse::<horae_core::types::ProjectType>()
            .map_err(|_| server_err("Invalid project_type"))?,
    )
    .bind(&currency)
    .bind(
        budget_kind
            .parse::<horae_core::types::BudgetKind>()
            .map_err(|_| server_err("Invalid budget_kind"))?,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Project not found".into(),
        code: NOT_FOUND,
        details: None,
    })
}

/// Activate or deactivate a project. Deactivated projects are hidden from
/// new-entry pickers but stay attached to existing time entries (FR-011).
#[server]
pub async fn set_project_active(
    project_id: String,
    active: bool,
) -> Result<Project, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let project_id: uuid::Uuid = project_id
        .parse()
        .map_err(|_| server_err("Invalid project_id"))?;
    sqlx::query_as::<_, Project>(
        "UPDATE projects SET active = $3
          WHERE id = $1 AND org_id = $2
         RETURNING id, org_id, client_id, code, name,
                   project_type, currency,
                   starts_on, ends_on,
                   budget_kind,
                   budget_amount_cents, budget_minutes, active, created_at",
    )
    .bind(project_id)
    .bind(manager.org_id)
    .bind(active)
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Project not found".into(),
        code: NOT_FOUND,
        details: None,
    })
}

// ── Tasks ────────────────────────────────────────────────────────────────────

/// Lists all active org-level tasks.
#[server]
pub async fn list_tasks() -> Result<Vec<Task>, ServerFnError> {
    let state = crate::state::global_state().await;

    let tasks = sqlx::query_as!(
        Task,
        "SELECT id, org_id, name, billable_default, default_rate_cents, active
         FROM tasks
         WHERE active = true
         ORDER BY name ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(tasks)
}

/// Lists tasks linked to a specific project via the `project_tasks` join table.
#[server]
pub async fn list_project_tasks(project_id: String) -> Result<Vec<Task>, ServerFnError> {
    let _user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let project_id: uuid::Uuid = project_id
        .parse()
        .map_err(|_| server_err("Invalid project_id"))?;

    sqlx::query_as!(
        Task,
        "SELECT t.id, t.org_id, t.name, t.billable_default, t.default_rate_cents, t.active
         FROM tasks t
         JOIN project_tasks pt ON t.id = pt.task_id
         WHERE pt.project_id = $1 AND t.active = true
         ORDER BY t.name",
        project_id,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)
}

#[server]
pub async fn create_task(name: String, billable_default: bool) -> Result<Task, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    sqlx::query_as!(
        Task,
        "INSERT INTO tasks (id, org_id, name, billable_default)
         VALUES ($1, $2, $3, $4)
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
        id,
        manager.org_id,
        name,
        billable_default,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)
}

#[server]
pub async fn update_task(
    task_id: String,
    name: String,
    billable_default: bool,
    default_rate_cents: Option<i64>,
) -> Result<Task, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let task_id: uuid::Uuid = task_id.parse().map_err(|_| server_err("Invalid task_id"))?;
    sqlx::query_as::<_, Task>(
        "UPDATE tasks
            SET name = $3, billable_default = $4, default_rate_cents = $5
          WHERE id = $1 AND org_id = $2
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
    )
    .bind(task_id)
    .bind(manager.org_id)
    .bind(&name)
    .bind(billable_default)
    .bind(default_rate_cents)
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Task not found".into(),
        code: NOT_FOUND,
        details: None,
    })
}

/// Activate or deactivate an org-level task. Deactivated tasks are hidden from
/// new-entry pickers but stay attached to existing time entries (FR-011).
#[server]
pub async fn set_task_active(task_id: String, active: bool) -> Result<Task, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let task_id: uuid::Uuid = task_id.parse().map_err(|_| server_err("Invalid task_id"))?;
    sqlx::query_as::<_, Task>(
        "UPDATE tasks SET active = $3
          WHERE id = $1 AND org_id = $2
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
    )
    .bind(task_id)
    .bind(manager.org_id)
    .bind(active)
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Task not found".into(),
        code: NOT_FOUND,
        details: None,
    })
}

/// Enable an org-level task on a project so it becomes loggable there. The
/// project-task link inherits the task's default billable flag; idempotent.
/// Both the project and the task must belong to the manager's organization.
#[server]
pub async fn link_project_task(project_id: String, task_id: String) -> Result<(), ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let project_id: uuid::Uuid = project_id
        .parse()
        .map_err(|_| server_err("Invalid project_id"))?;
    let task_id: uuid::Uuid = task_id.parse().map_err(|_| server_err("Invalid task_id"))?;

    let result = sqlx::query(
        "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents)
         SELECT p.id, t.id, t.billable_default, t.default_rate_cents
           FROM projects p
           JOIN tasks t ON t.org_id = p.org_id
          WHERE p.id = $1 AND t.id = $2 AND p.org_id = $3
         ON CONFLICT (project_id, task_id) DO NOTHING",
    )
    .bind(project_id)
    .bind(task_id)
    .bind(manager.org_id)
    .execute(&state.db)
    .await
    .map_err(server_err)?;

    // No row inserted and no existing link means the project/task pair was not
    // found in this org (the SELECT matched nothing).
    if result.rows_affected() == 0 {
        let linked = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM project_tasks WHERE project_id = $1 AND task_id = $2)",
        )
        .bind(project_id)
        .bind(task_id)
        .fetch_one(&state.db)
        .await
        .map_err(server_err)?;
        if !linked {
            return Err(ServerFnError::ServerError {
                message: "Project or task not found in this organization".into(),
                code: NOT_FOUND,
                details: None,
            });
        }
    }
    Ok(())
}

// ── Assignments ─────────────────────────────────────────────────────────────

#[server]
pub async fn list_assignments(project_id: String) -> Result<Vec<Assignment>, ServerFnError> {
    let _user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let project_id: uuid::Uuid = project_id
        .parse()
        .map_err(|_| server_err("Invalid project_id"))?;
    sqlx::query_as!(
        Assignment,
        r#"SELECT id, project_id, user_id, role as "role: ProjectRole", rate_cents,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM assignments WHERE project_id = $1 ORDER BY created_at"#,
        project_id,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)
}

#[server]
pub async fn create_assignment(
    project_id: String,
    user_id: String,
    role: String,
) -> Result<Assignment, ServerFnError> {
    let _admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    let project_id: uuid::Uuid = project_id
        .parse()
        .map_err(|_| server_err("Invalid project_id"))?;
    let user_id: uuid::Uuid = user_id.parse().map_err(|_| server_err("Invalid user_id"))?;
    let pr = role
        .parse::<ProjectRole>()
        .map_err(|_| server_err("Invalid role"))?;
    sqlx::query_as!(
        Assignment,
        r#"INSERT INTO assignments (id, project_id, user_id, role)
         VALUES ($1, $2, $3, $4)
         RETURNING id, project_id, user_id, role as "role: ProjectRole", rate_cents,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        project_id,
        user_id,
        pr as ProjectRole,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)
}

#[server]
pub async fn delete_assignment(assignment_id: String) -> Result<(), ServerFnError> {
    let _admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let id: uuid::Uuid = assignment_id
        .parse()
        .map_err(|_| server_err("Invalid assignment_id"))?;
    sqlx::query!("DELETE FROM assignments WHERE id = $1", id)
        .execute(&state.db)
        .await
        .map_err(server_err)?;
    Ok(())
}

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
    state.plugins.dispatch(crate::plugin::AppEvent::InvoiceCreated {
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
        state.plugins.dispatch(crate::plugin::AppEvent::InvoiceSent {
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

    Ok(invoice)
}

// ── Organization branding ─────────────────────────────────────────────────────

#[server]
pub async fn get_org_branding() -> Result<OrgBranding, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;

    let branding = sqlx::query_as!(
        OrgBranding,
        r#"SELECT provider_name, provider_address, provider_tax_id,
                  provider_email, provider_phone,
                  bank_name, bank_iban, bank_bic, bank_routing, bank_account,
                  invoice_notes, invoice_payment_terms
           FROM organizations WHERE id = $1"#,
        manager.org_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    Ok(branding)
}

#[server]
pub async fn update_org_branding(branding: OrgBranding) -> Result<OrgBranding, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;

    let branding = sqlx::query_as!(
        OrgBranding,
        r#"UPDATE organizations SET
             provider_name = $2,
             provider_address = $3,
             provider_tax_id = $4,
             provider_email = $5,
             provider_phone = $6,
             bank_name = $7,
             bank_iban = $8,
             bank_bic = $9,
             bank_routing = $10,
             bank_account = $11,
             invoice_notes = $12,
             invoice_payment_terms = $13
           WHERE id = $1
           RETURNING provider_name, provider_address, provider_tax_id,
                     provider_email, provider_phone,
                     bank_name, bank_iban, bank_bic, bank_routing, bank_account,
                     invoice_notes, invoice_payment_terms"#,
        manager.org_id,
        branding.provider_name,
        branding.provider_address,
        branding.provider_tax_id,
        branding.provider_email,
        branding.provider_phone,
        branding.bank_name,
        branding.bank_iban,
        branding.bank_bic,
        branding.bank_routing,
        branding.bank_account,
        branding.invoice_notes,
        branding.invoice_payment_terms,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    Ok(branding)
}

// ── Users ─────────────────────────────────────────────────────────────────────

/// List users. Any authenticated user can list active users (for pickers);
/// pass `include_inactive = true` to also see deactivated accounts (admin only).
#[server]
pub async fn list_users(include_inactive: bool) -> Result<Vec<User>, ServerFnError> {
    if include_inactive {
        let _admin = require_admin().await?;
    } else {
        let _uid = session_user_id().await?;
    }
    let state = crate::state::global_state().await;

    let users = sqlx::query_as!(
        User,
        r#"SELECT id, org_id, email, name, oidc_subject,
                org_role as "org_role: OrgRole",
                cost_rate_cents, billable_rate_cents, active,
                created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM users
         WHERE ($1::bool OR active = true)
         ORDER BY name ASC"#,
        include_inactive,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(users)
}

/// Create a new user account. Requires admin role.
#[server]
pub async fn create_user(email: String, name: String, role: String) -> Result<User, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    let org_role = role
        .parse::<OrgRole>()
        .map_err(|_| server_err("Invalid role (use admin, manager, or member)"))?;

    sqlx::query_as!(
        User,
        r#"INSERT INTO users (id, org_id, email, name, org_role)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, org_id, email, name, oidc_subject,
                   org_role as "org_role: OrgRole",
                   cost_rate_cents, billable_rate_cents, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        admin.org_id,
        email,
        name,
        org_role as OrgRole,
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("users_email_key") {
            ServerFnError::ServerError {
                message: "A user with this email already exists".into(),
                code: CONFLICT,
                details: None,
            }
        } else {
            server_err(e)
        }
    })
}

/// Change a user's organization role. Requires admin role.
#[server]
pub async fn set_user_role(user_id: String, role: String) -> Result<User, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let user_id: uuid::Uuid = user_id.parse().map_err(|_| server_err("Invalid user_id"))?;
    let org_role = role
        .parse::<OrgRole>()
        .map_err(|_| server_err("Invalid role (use admin, manager, or member)"))?;

    sqlx::query_as!(
        User,
        r#"UPDATE users SET org_role = $3
         WHERE id = $1 AND org_id = $2
         RETURNING id, org_id, email, name, oidc_subject,
                   org_role as "org_role: OrgRole",
                   cost_rate_cents, billable_rate_cents, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        user_id,
        admin.org_id,
        org_role as OrgRole,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "User not found".into(),
        code: NOT_FOUND,
        details: None,
    })
}

/// Activate or deactivate a user account. Deactivated users cannot sign in
/// but their historical time entries are preserved (FR-002).
#[server]
pub async fn set_user_active(user_id: String, active: bool) -> Result<User, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let user_id: uuid::Uuid = user_id.parse().map_err(|_| server_err("Invalid user_id"))?;

    sqlx::query_as!(
        User,
        r#"UPDATE users SET active = $3
         WHERE id = $1 AND org_id = $2
         RETURNING id, org_id, email, name, oidc_subject,
                   org_role as "org_role: OrgRole",
                   cost_rate_cents, billable_rate_cents, active,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        user_id,
        admin.org_id,
        active,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "User not found".into(),
        code: NOT_FOUND,
        details: None,
    })
}

// ── Approvals (M7) ──────────────────────────────────────────────────────────

/// Submit a week of time entries for approval.
/// Transitions all 'open' entries in [week_start, week_start+6] to 'submitted'
/// and creates an approval row.
#[server]
pub async fn submit_week(week_start: String) -> Result<Approval, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;

    let ws: chrono::NaiveDate = week_start
        .parse()
        .map_err(|_| server_err("Invalid week_start (use YYYY-MM-DD)"))?;
    let we = ws + chrono::Duration::days(6);

    // Get user's org_id
    let user_row = sqlx::query!("SELECT org_id FROM users WHERE id = $1", user_id)
        .fetch_one(&state.db)
        .await
        .map_err(server_err)?;
    let org_id = user_row.org_id;

    // Fetch org rounding config
    let org_row = sqlx::query!(
        r#"SELECT round_minutes, round_dir as "round_dir: RoundDir" FROM organizations WHERE id = $1"#,
        org_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;
    let round_min = org_row.round_minutes;
    let round_dir = org_row.round_dir;

    // Apply rounding per entry if rounding is configured
    if round_min > 0 {
        let entries = sqlx::query!(
            "SELECT id, minutes FROM time_entries
             WHERE user_id = $1 AND spent_date BETWEEN $2 AND $3 AND state = $4",
            user_id,
            ws as chrono::NaiveDate,
            we as chrono::NaiveDate,
            EntryState::Open as EntryState,
        )
        .fetch_all(&state.db)
        .await
        .map_err(server_err)?;

        for entry in &entries {
            let rounded =
                horae_core::rounding::round(entry.minutes as u32, round_min as u32, round_dir)
                    as i32;
            sqlx::query!(
                "UPDATE time_entries SET rounded_minutes = $1 WHERE id = $2",
                rounded,
                entry.id,
            )
            .execute(&state.db)
            .await
            .map_err(server_err)?;
        }
    }

    // Transition open entries to submitted, using COALESCE so entries without
    // explicit rounding (round_min=0) still get rounded_minutes set to minutes
    let result = sqlx::query!(
        "UPDATE time_entries
         SET state = $4,
             rounded_minutes = COALESCE(rounded_minutes, minutes),
             updated_at = now()
         WHERE user_id = $1
           AND spent_date BETWEEN $2 AND $3
           AND state = $5",
        user_id,
        ws as chrono::NaiveDate,
        we as chrono::NaiveDate,
        EntryState::Submitted as EntryState,
        EntryState::Open as EntryState,
    )
    .execute(&state.db)
    .await
    .map_err(server_err)?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::ServerError {
            message: "No open entries found for this week".into(),
            code: NOT_FOUND,
            details: None,
        });
    }

    // Create approval row
    let id = uuid::Uuid::now_v7();
    let approval = sqlx::query_as!(
        Approval,
        r#"INSERT INTO approvals (id, org_id, user_id, period_start, period_end, state)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (user_id, period_start) DO UPDATE
           SET state = $6, submitted_at = now()
         RETURNING id, org_id, user_id,
                   period_start as "period_start: chrono::NaiveDate",
                   period_end as "period_end: chrono::NaiveDate",
                   state as "state: EntryState",
                   submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                   approved_by,
                   approved_at as "approved_at: chrono::DateTime<chrono::Utc>""#,
        id,
        org_id,
        user_id,
        ws as chrono::NaiveDate,
        we as chrono::NaiveDate,
        EntryState::Submitted as EntryState,
    )
    .fetch_one(&state.db)
    .await
    .map_err(server_err)?;

    Ok(approval)
}

/// List approvals, optionally filtered by state. Requires manager role.
#[server]
pub async fn list_approvals(status: Option<String>) -> Result<Vec<Approval>, ServerFnError> {
    let _manager = require_manager().await?;
    let state = crate::state::global_state().await;

    let state_filter: Option<EntryState> = status
        .map(|s| {
            s.parse::<EntryState>()
                .map_err(|_| server_err("Invalid status"))
        })
        .transpose()?;

    let approvals = sqlx::query_as!(
        Approval,
        r#"SELECT id, org_id, user_id,
                period_start as "period_start: chrono::NaiveDate",
                period_end as "period_end: chrono::NaiveDate",
                state as "state: EntryState",
                submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                approved_by,
                approved_at as "approved_at: chrono::DateTime<chrono::Utc>"
         FROM approvals
         WHERE ($1::entry_state IS NULL OR state = $1)
         ORDER BY period_start DESC"#,
        state_filter as Option<EntryState>,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(approvals)
}

/// Approve a submitted week. Requires manager role.
#[server]
pub async fn approve_submission(approval_id: String) -> Result<Approval, ServerFnError> {
    let manager = require_manager().await?;

    if !horae_core::state::can_transition(
        EntryState::Submitted,
        EntryState::Approved,
        manager.org_role,
    ) {
        return Err(ServerFnError::ServerError {
            message: "Insufficient role to approve submissions".into(),
            code: FORBIDDEN,
            details: None,
        });
    }

    let state = crate::state::global_state().await;
    let approval_id: uuid::Uuid = approval_id
        .parse()
        .map_err(|_| server_err("Invalid approval_id"))?;

    // Update approval row
    let approval = sqlx::query_as!(
        Approval,
        r#"UPDATE approvals
         SET state = $3,
             approved_by = $2,
             approved_at = now()
         WHERE id = $1 AND state = $4
         RETURNING id, org_id, user_id,
                   period_start as "period_start: chrono::NaiveDate",
                   period_end as "period_end: chrono::NaiveDate",
                   state as "state: EntryState",
                   submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                   approved_by,
                   approved_at as "approved_at: chrono::DateTime<chrono::Utc>""#,
        approval_id,
        manager.id,
        EntryState::Approved as EntryState,
        EntryState::Submitted as EntryState,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Approval not found or not in 'submitted' state".into(),
        code: NOT_FOUND,
        details: None,
    })?;

    // Transition corresponding time entries to approved
    sqlx::query!(
        "UPDATE time_entries
         SET state = $4, updated_at = now()
         WHERE user_id = $1
           AND spent_date BETWEEN $2 AND $3
           AND state = $5",
        approval.user_id,
        approval.period_start as chrono::NaiveDate,
        approval.period_end as chrono::NaiveDate,
        EntryState::Approved as EntryState,
        EntryState::Submitted as EntryState,
    )
    .execute(&state.db)
    .await
    .map_err(server_err)?;

    Ok(approval)
}

/// Reject a submitted week. Requires manager role.
/// Reopens the time entries and deletes the approval row.
#[server]
pub async fn reject_submission(approval_id: String) -> Result<(), ServerFnError> {
    let manager = require_manager().await?;

    if !horae_core::state::can_transition(EntryState::Submitted, EntryState::Open, manager.org_role)
    {
        return Err(ServerFnError::ServerError {
            message: "Insufficient role to reject submissions".into(),
            code: FORBIDDEN,
            details: None,
        });
    }

    let state = crate::state::global_state().await;
    let approval_id: uuid::Uuid = approval_id
        .parse()
        .map_err(|_| server_err("Invalid approval_id"))?;

    // Fetch the approval to know user + period
    let approval = sqlx::query_as!(
        Approval,
        r#"SELECT id, org_id, user_id,
                period_start as "period_start: chrono::NaiveDate",
                period_end as "period_end: chrono::NaiveDate",
                state as "state: EntryState",
                submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                approved_by,
                approved_at as "approved_at: chrono::DateTime<chrono::Utc>"
         FROM approvals WHERE id = $1"#,
        approval_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Approval not found".into(),
        code: NOT_FOUND,
        details: None,
    })?;

    // Reopen entries
    sqlx::query!(
        "UPDATE time_entries
         SET state = $4, rounded_minutes = NULL, updated_at = now()
         WHERE user_id = $1
           AND spent_date BETWEEN $2 AND $3
           AND state = $5",
        approval.user_id,
        approval.period_start as chrono::NaiveDate,
        approval.period_end as chrono::NaiveDate,
        EntryState::Open as EntryState,
        EntryState::Submitted as EntryState,
    )
    .execute(&state.db)
    .await
    .map_err(server_err)?;

    // Delete the approval row (per schema: "reject deletes the row")
    sqlx::query!("DELETE FROM approvals WHERE id = $1", approval_id)
        .execute(&state.db)
        .await
        .map_err(server_err)?;

    Ok(())
}

// ── Reports (M8) ────────────────────────────────────────────────────────────

/// Grouped time report. Groups by "project", "task", "client", or "person".
#[server]
pub async fn report_time(
    from: String,
    to: String,
    group_by: String,
) -> Result<Vec<ReportRow>, ServerFnError> {
    let _user_id = session_user_id().await?;
    let state = crate::state::global_state().await;

    let from_date: chrono::NaiveDate = from
        .parse()
        .map_err(|_| server_err("Invalid from date (use YYYY-MM-DD)"))?;
    let to_date: chrono::NaiveDate = to
        .parse()
        .map_err(|_| server_err("Invalid to date (use YYYY-MM-DD)"))?;

    // Fetch detailed rows and group in Rust for flexibility
    let entries = sqlx::query_as!(
        DetailedReportRow,
        r#"SELECT te.spent_date as "spent_date: chrono::NaiveDate",
                p.name AS project_name, t.name AS task_name,
                u.name AS user_name, te.minutes, te.rounded_minutes, te.billable, te.notes
         FROM time_entries te
         JOIN projects p ON te.project_id = p.id
         JOIN tasks t ON te.task_id = t.id
         JOIN users u ON te.user_id = u.id
         WHERE te.spent_date BETWEEN $1 AND $2
         ORDER BY te.spent_date"#,
        from_date as chrono::NaiveDate,
        to_date as chrono::NaiveDate,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    // Group by the requested dimension
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, (i64, i64, i64)> = BTreeMap::new();
    for e in &entries {
        let label = match group_by.as_str() {
            "task" => e.task_name.clone(),
            "client" => e.project_name.clone(), // project serves as proxy until we join clients
            "person" => e.user_name.clone(),
            _ => e.project_name.clone(), // default: "project"
        };
        let agg = groups.entry(label).or_insert((0, 0, 0));
        agg.0 += e.minutes as i64;
        agg.1 += e.rounded_minutes.unwrap_or(e.minutes) as i64;
        if e.billable {
            agg.2 += e.rounded_minutes.unwrap_or(e.minutes) as i64;
        }
    }

    let rows: Vec<ReportRow> = groups
        .into_iter()
        .map(|(label, (total, rounded, billable))| ReportRow {
            label,
            total_minutes: total,
            rounded_minutes: rounded,
            billable_minutes: billable,
        })
        .collect();

    Ok(rows)
}

/// Detailed (per-entry) report for the given date range.
#[server]
pub async fn report_detailed(
    from: String,
    to: String,
) -> Result<Vec<DetailedReportRow>, ServerFnError> {
    let _user_id = session_user_id().await?;
    let state = crate::state::global_state().await;

    let from_date: chrono::NaiveDate = from
        .parse()
        .map_err(|_| server_err("Invalid from date (use YYYY-MM-DD)"))?;
    let to_date: chrono::NaiveDate = to
        .parse()
        .map_err(|_| server_err("Invalid to date (use YYYY-MM-DD)"))?;

    let entries = sqlx::query_as!(
        DetailedReportRow,
        r#"SELECT te.spent_date as "spent_date: chrono::NaiveDate",
                p.name AS project_name, t.name AS task_name,
                u.name AS user_name, te.minutes, te.rounded_minutes, te.billable, te.notes
         FROM time_entries te
         JOIN projects p ON te.project_id = p.id
         JOIN tasks t ON te.task_id = t.id
         JOIN users u ON te.user_id = u.id
         WHERE te.spent_date BETWEEN $1 AND $2
         ORDER BY te.spent_date, p.name, t.name"#,
        from_date as chrono::NaiveDate,
        to_date as chrono::NaiveDate,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(entries)
}

// ── Plugins ────────────────────────────────────────────────────────────────

/// Collect dashboard widgets from all loaded plugins (FR-022).
#[server]
pub async fn get_plugin_widgets() -> Result<Vec<PluginWidget>, ServerFnError> {
    let state = crate::state::global_state().await;
    let widgets = state.plugins.collect_widgets().await;
    Ok(widgets
        .into_iter()
        .map(|w| PluginWidget {
            plugin_name: w.plugin_name,
            title: w.title,
            body: w.body,
        })
        .collect())
}

/// A dashboard widget contributed by a plugin, serializable for the SPA.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginWidget {
    pub plugin_name: String,
    pub title: String,
    pub body: String,
}
