use dioxus::prelude::*;

use crate::models::{Approval, Assignment, Client, Invoice, Project, Task, TimeEntry, User};

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

#[cfg(feature = "server")]
async fn require_admin() -> Result<crate::models::User, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let user = sqlx::query_as::<_, crate::models::User>(
        "SELECT id, org_id, email, name, oidc_subject, org_role::text AS org_role,
                cost_rate_cents, billable_rate_cents, active, created_at
         FROM users WHERE id = $1 AND active = true",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| server_err(e))?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "User not found".into(),
        code: 404,
        details: None,
    })?;

    if !user.is_admin() {
        return Err(ServerFnError::ServerError {
            message: "Admin access required".into(),
            code: 403,
            details: None,
        });
    }
    Ok(user)
}

#[cfg(feature = "server")]
async fn require_manager() -> Result<crate::models::User, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let user = sqlx::query_as::<_, crate::models::User>(
        "SELECT id, org_id, email, name, oidc_subject, org_role::text AS org_role,
                cost_rate_cents, billable_rate_cents, active, created_at
         FROM users WHERE id = $1 AND active = true",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| server_err(e))?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "User not found".into(),
        code: 404,
        details: None,
    })?;

    if !user.is_manager_or_above() {
        return Err(ServerFnError::ServerError {
            message: "Manager access required".into(),
            code: 403,
            details: None,
        });
    }
    Ok(user)
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
    let session_uid = session_user_id().await?;
    let state = crate::state::global_state().await;
    let limit = limit.unwrap_or(50);

    let project_filter: Option<uuid::Uuid> = match project_id {
        Some(ref s) => Some(s.parse().map_err(|_| server_err("Invalid project_id"))?),
        None => None,
    };
    let date_filter: Option<chrono::NaiveDate> = match date_from {
        Some(ref s) => Some(s.parse().map_err(|_| server_err("Invalid date_from (use YYYY-MM-DD)"))?),
        None => None,
    };

    let entries = sqlx::query_as::<_, TimeEntry>(
        "SELECT id, org_id, user_id, project_id, task_id, spent_date,
                minutes, rounded_minutes, notes, billable, is_running, started_at,
                state::text AS state, invoice_id, created_at, updated_at
         FROM time_entries
         WHERE user_id = $1
           AND ($2::uuid IS NULL OR project_id = $2)
           AND ($3::date IS NULL OR spent_date >= $3)
         ORDER BY spent_date DESC, created_at DESC
         LIMIT $4",
    )
    .bind(session_uid)
    .bind(project_filter)
    .bind(date_filter)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(|e| server_err(e))?;

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
    let task_id: uuid::Uuid = task_id
        .parse()
        .map_err(|_| server_err("Invalid task_id"))?;

    // Get user's org_id
    let user = sqlx::query_as::<_, User>(
        "SELECT id, org_id, email, name, oidc_subject, org_role::text AS org_role,
                cost_rate_cents, billable_rate_cents, active, created_at
         FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    // Check no timer already running
    let existing = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM time_entries WHERE user_id = $1 AND is_running = true)",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    if existing {
        return Err(ServerFnError::ServerError {
            message: "A timer is already running. Stop it first.".into(),
            code: 409,
            details: None,
        });
    }

    let id = uuid::Uuid::now_v7();
    let today = chrono::Utc::now().date_naive();

    sqlx::query_as::<_, TimeEntry>(
        "INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, notes, billable, is_running, started_at, state)
         VALUES ($1, $2, $3, $4, $5, $6, 0, $7, true, true, now(), 'open'::entry_state)
         RETURNING id, org_id, user_id, project_id, task_id, spent_date,
                   minutes, rounded_minutes, notes, billable, is_running, started_at,
                   state::text AS state, invoice_id, created_at, updated_at",
    )
    .bind(id)
    .bind(user.org_id)
    .bind(user_id)
    .bind(project_id)
    .bind(task_id)
    .bind(today)
    .bind(&notes)
    .fetch_one(&state.db)
    .await
    .map_err(|e| server_err(e))
}

/// Stop a running timer and record elapsed minutes.
#[server]
pub async fn stop_timer(entry_id: String) -> Result<TimeEntry, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let entry_id: uuid::Uuid = entry_id
        .parse()
        .map_err(|_| server_err("Invalid entry_id"))?;

    sqlx::query_as::<_, TimeEntry>(
        "UPDATE time_entries
         SET is_running = false,
             minutes = GREATEST(1, EXTRACT(EPOCH FROM (now() - started_at))::int / 60),
             started_at = NULL,
             updated_at = now()
         WHERE id = $1 AND user_id = $2 AND is_running = true
         RETURNING id, org_id, user_id, project_id, task_id, spent_date,
                   minutes, rounded_minutes, notes, billable, is_running, started_at,
                   state::text AS state, invoice_id, created_at, updated_at",
    )
    .bind(entry_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| server_err(e))?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "No running timer found for this entry".into(),
        code: 404,
        details: None,
    })
}

/// Return the currently running timer for the authenticated user, if any.
#[server]
pub async fn get_current_timer() -> Result<Option<TimeEntry>, ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;

    let entry = sqlx::query_as::<_, TimeEntry>(
        "SELECT id, org_id, user_id, project_id, task_id, spent_date,
                minutes, rounded_minutes, notes, billable, is_running, started_at,
                state::text AS state, invoice_id, created_at, updated_at
         FROM time_entries
         WHERE user_id = $1 AND is_running = true
         LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| server_err(e))?;

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
    let task_id: uuid::Uuid = task_id
        .parse()
        .map_err(|_| server_err("Invalid task_id"))?;
    let spent_date: chrono::NaiveDate = spent_date
        .parse()
        .map_err(|_| server_err("Invalid date (use YYYY-MM-DD)"))?;

    let (org_id,): (uuid::Uuid,) =
        sqlx::query_as("SELECT org_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| server_err(e))?;

    let id = uuid::Uuid::now_v7();

    sqlx::query_as::<_, TimeEntry>(
        "INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, notes, billable, is_running, state)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, false, 'open'::entry_state)
         RETURNING id, org_id, user_id, project_id, task_id, spent_date,
                   minutes, rounded_minutes, notes, billable, is_running, started_at,
                   state::text AS state, invoice_id, created_at, updated_at",
    )
    .bind(id)
    .bind(org_id)
    .bind(user_id)
    .bind(project_id)
    .bind(task_id)
    .bind(spent_date)
    .bind(minutes)
    .bind(&notes)
    .bind(billable)
    .fetch_one(&state.db)
    .await
    .map_err(|e| server_err(e))
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

    sqlx::query_as::<_, TimeEntry>(
        "UPDATE time_entries
         SET minutes = $3, notes = $4, billable = $5, updated_at = now()
         WHERE id = $1 AND user_id = $2 AND state = 'open'
         RETURNING id, org_id, user_id, project_id, task_id, spent_date,
                   minutes, rounded_minutes, notes, billable, is_running, started_at,
                   state::text AS state, invoice_id, created_at, updated_at",
    )
    .bind(entry_id)
    .bind(user_id)
    .bind(minutes)
    .bind(&notes)
    .bind(billable)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| server_err(e))?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Entry not found or is locked (not in 'open' state)".into(),
        code: 409,
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

    let result = sqlx::query(
        "DELETE FROM time_entries WHERE id = $1 AND user_id = $2 AND state = 'open'",
    )
    .bind(entry_id)
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::ServerError {
            message: "Entry not found or is locked (not in 'open' state)".into(),
            code: 409,
            details: None,
        });
    }

    Ok(())
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

#[server]
pub async fn create_client(
    name: String,
    currency: String,
    address: Option<String>,
    tax_id: Option<String>,
) -> Result<Client, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    sqlx::query_as::<_, Client>(
        "INSERT INTO clients (id, org_id, name, currency, address, tax_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, org_id, name, currency, address, tax_id, active, created_at",
    )
    .bind(id)
    .bind(admin.org_id)
    .bind(&name)
    .bind(&currency)
    .bind(&address)
    .bind(&tax_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| server_err(e))
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

#[server]
pub async fn create_project(
    client_id: String,
    name: String,
    project_type: String,
    currency: String,
    budget_kind: String,
) -> Result<Project, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    let client_id: uuid::Uuid = client_id
        .parse()
        .map_err(|_| server_err("Invalid client_id"))?;
    sqlx::query_as::<_, Project>(
        "INSERT INTO projects (id, org_id, client_id, name, project_type, currency, budget_kind)
         VALUES ($1, $2, $3, $4, $5::project_type, $6, $7::budget_kind)
         RETURNING id, org_id, client_id, code, name,
                   project_type::text AS project_type, currency,
                   starts_on, ends_on,
                   budget_kind::text AS budget_kind,
                   budget_amount_cents, budget_minutes, active, created_at",
    )
    .bind(id)
    .bind(admin.org_id)
    .bind(client_id)
    .bind(&name)
    .bind(&project_type)
    .bind(&currency)
    .bind(&budget_kind)
    .fetch_one(&state.db)
    .await
    .map_err(|e| server_err(e))
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

#[server]
pub async fn create_task(
    name: String,
    billable_default: bool,
) -> Result<Task, ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    sqlx::query_as::<_, Task>(
        "INSERT INTO tasks (id, org_id, name, billable_default)
         VALUES ($1, $2, $3, $4)
         RETURNING id, org_id, name, billable_default, default_rate_cents, active",
    )
    .bind(id)
    .bind(admin.org_id)
    .bind(&name)
    .bind(billable_default)
    .fetch_one(&state.db)
    .await
    .map_err(|e| server_err(e))
}

// ── Assignments ─────────────────────────────────────────────────────────────

#[server]
pub async fn list_assignments(project_id: String) -> Result<Vec<Assignment>, ServerFnError> {
    let _user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let project_id: uuid::Uuid = project_id
        .parse()
        .map_err(|_| server_err("Invalid project_id"))?;
    sqlx::query_as::<_, Assignment>(
        "SELECT id, project_id, user_id, role::text AS role, rate_cents, created_at
         FROM assignments WHERE project_id = $1 ORDER BY created_at",
    )
    .bind(project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| server_err(e))
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
    let user_id: uuid::Uuid = user_id
        .parse()
        .map_err(|_| server_err("Invalid user_id"))?;
    sqlx::query_as::<_, Assignment>(
        "INSERT INTO assignments (id, project_id, user_id, role)
         VALUES ($1, $2, $3, $4::project_role)
         RETURNING id, project_id, user_id, role::text AS role, rate_cents, created_at",
    )
    .bind(id)
    .bind(project_id)
    .bind(user_id)
    .bind(&role)
    .fetch_one(&state.db)
    .await
    .map_err(|e| server_err(e))
}

#[server]
pub async fn delete_assignment(assignment_id: String) -> Result<(), ServerFnError> {
    let _admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let id: uuid::Uuid = assignment_id
        .parse()
        .map_err(|_| server_err("Invalid assignment_id"))?;
    sqlx::query("DELETE FROM assignments WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| server_err(e))?;
    Ok(())
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
    let (org_id,): (uuid::Uuid,) =
        sqlx::query_as("SELECT org_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| server_err(e))?;

    // Transition open entries to submitted, snapshotting rounded_minutes
    let result = sqlx::query(
        "UPDATE time_entries
         SET state = 'submitted'::entry_state,
             rounded_minutes = minutes,
             updated_at = now()
         WHERE user_id = $1
           AND spent_date BETWEEN $2 AND $3
           AND state = 'open'",
    )
    .bind(user_id)
    .bind(ws)
    .bind(we)
    .execute(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    if result.rows_affected() == 0 {
        return Err(ServerFnError::ServerError {
            message: "No open entries found for this week".into(),
            code: 404,
            details: None,
        });
    }

    // Create approval row
    let id = uuid::Uuid::now_v7();
    let approval = sqlx::query_as::<_, Approval>(
        "INSERT INTO approvals (id, org_id, user_id, period_start, period_end, state)
         VALUES ($1, $2, $3, $4, $5, 'submitted'::entry_state)
         ON CONFLICT (user_id, period_start) DO UPDATE
           SET state = 'submitted'::entry_state, submitted_at = now()
         RETURNING id, org_id, user_id, period_start, period_end,
                   state::text AS state, submitted_at, approved_by, approved_at",
    )
    .bind(id)
    .bind(org_id)
    .bind(user_id)
    .bind(ws)
    .bind(we)
    .fetch_one(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    Ok(approval)
}

/// List approvals, optionally filtered by state. Requires manager role.
#[server]
pub async fn list_approvals(status: Option<String>) -> Result<Vec<Approval>, ServerFnError> {
    let _manager = require_manager().await?;
    let state = crate::state::global_state().await;

    let approvals = sqlx::query_as::<_, Approval>(
        "SELECT id, org_id, user_id, period_start, period_end,
                state::text AS state, submitted_at, approved_by, approved_at
         FROM approvals
         WHERE ($1::text IS NULL OR state::text = $1)
         ORDER BY period_start DESC",
    )
    .bind(&status)
    .fetch_all(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    Ok(approvals)
}

/// Approve a submitted week. Requires manager role.
#[server]
pub async fn approve_submission(approval_id: String) -> Result<Approval, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let approval_id: uuid::Uuid = approval_id
        .parse()
        .map_err(|_| server_err("Invalid approval_id"))?;

    // Update approval row
    let approval = sqlx::query_as::<_, Approval>(
        "UPDATE approvals
         SET state = 'approved'::entry_state,
             approved_by = $2,
             approved_at = now()
         WHERE id = $1 AND state = 'submitted'::entry_state
         RETURNING id, org_id, user_id, period_start, period_end,
                   state::text AS state, submitted_at, approved_by, approved_at",
    )
    .bind(approval_id)
    .bind(manager.id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| server_err(e))?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Approval not found or not in 'submitted' state".into(),
        code: 404,
        details: None,
    })?;

    // Transition corresponding time entries to approved
    sqlx::query(
        "UPDATE time_entries
         SET state = 'approved'::entry_state, updated_at = now()
         WHERE user_id = $1
           AND spent_date BETWEEN $2 AND $3
           AND state = 'submitted'",
    )
    .bind(approval.user_id)
    .bind(approval.period_start)
    .bind(approval.period_end)
    .execute(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    Ok(approval)
}

/// Reject a submitted week. Requires manager role.
/// Reopens the time entries and deletes the approval row.
#[server]
pub async fn reject_submission(approval_id: String) -> Result<(), ServerFnError> {
    let _manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let approval_id: uuid::Uuid = approval_id
        .parse()
        .map_err(|_| server_err("Invalid approval_id"))?;

    // Fetch the approval to know user + period
    let approval = sqlx::query_as::<_, Approval>(
        "SELECT id, org_id, user_id, period_start, period_end,
                state::text AS state, submitted_at, approved_by, approved_at
         FROM approvals WHERE id = $1",
    )
    .bind(approval_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| server_err(e))?
    .ok_or_else(|| ServerFnError::ServerError {
        message: "Approval not found".into(),
        code: 404,
        details: None,
    })?;

    // Reopen entries
    sqlx::query(
        "UPDATE time_entries
         SET state = 'open'::entry_state, rounded_minutes = NULL, updated_at = now()
         WHERE user_id = $1
           AND spent_date BETWEEN $2 AND $3
           AND state = 'submitted'",
    )
    .bind(approval.user_id)
    .bind(approval.period_start)
    .bind(approval.period_end)
    .execute(&state.db)
    .await
    .map_err(|e| server_err(e))?;

    // Delete the approval row (per schema: "reject deletes the row")
    sqlx::query("DELETE FROM approvals WHERE id = $1")
        .bind(approval_id)
        .execute(&state.db)
        .await
        .map_err(|e| server_err(e))?;

    Ok(())
}
