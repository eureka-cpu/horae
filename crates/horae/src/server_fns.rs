use dioxus::prelude::*;
#[cfg(feature = "server")]
pub(crate) use horae_core::types::{
    BudgetKind, EntryState, InvoiceStatus, OrgRole, ProjectRole, ProjectType, RoundDir,
};

#[cfg(feature = "server")]
pub(crate) use crate::models::InvoiceLine;
pub(crate) use crate::models::{
    Approval, Assignment, Client, DetailedReportRow, Invoice, InvoiceWithLines, OrgBranding,
    Project, ProjectSpend, ReportRow, Task, TimeEntry, User,
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
pub(crate) use status::*;

/// Extract the session user UUID from the request context.
/// Returns `Err(401)` if the session has no user (not logged in).
#[cfg(feature = "server")]
pub(crate) async fn session_user_id() -> Result<uuid::Uuid, ServerFnError> {
    use tower_sessions::Session;

    // `FullstackContext::extract` pulls the Session from the Axum request extensions
    // injected by the SessionManagerLayer wrapping the server.
    // Session implements FromRequestParts for any S, so M is inferred as the
    // axum via::Parts marker type.
    let session: Session = dioxus_fullstack::FullstackContext::extract::<Session, _>().await?;

    crate::auth::session::get_session_user_id(&session)
        .await
        .ok_or_else(|| unauthorized("Not authenticated — please sign in."))
}

/// Build a `ServerFnError::ServerError` with a named status code, so error
/// paths read as `not_found("…")` rather than a five-line struct literal.
#[cfg(feature = "server")]
pub(crate) fn err(code: u16, msg: impl std::fmt::Display) -> ServerFnError {
    ServerFnError::ServerError {
        message: msg.to_string(),
        code,
        details: None,
    }
}

#[cfg(feature = "server")]
pub(crate) fn server_err(msg: impl std::fmt::Display) -> ServerFnError {
    err(INTERNAL_ERROR, msg)
}
#[cfg(feature = "server")]
pub(crate) fn not_found(msg: impl std::fmt::Display) -> ServerFnError {
    err(NOT_FOUND, msg)
}
#[cfg(feature = "server")]
pub(crate) fn forbidden(msg: impl std::fmt::Display) -> ServerFnError {
    err(FORBIDDEN, msg)
}
#[cfg(feature = "server")]
pub(crate) fn conflict(msg: impl std::fmt::Display) -> ServerFnError {
    err(CONFLICT, msg)
}
#[cfg(feature = "server")]
pub(crate) fn unauthorized(msg: impl std::fmt::Display) -> ServerFnError {
    err(UNAUTHORIZED, msg)
}

/// Parse a UUID argument, mapping a malformed value to a clear error naming the
/// field (e.g. `parse_uuid(&entry_id, "entry_id")`).
#[cfg(feature = "server")]
pub(crate) fn parse_uuid(s: &str, field: &str) -> Result<uuid::Uuid, ServerFnError> {
    s.parse()
        .map_err(|_| server_err(format!("Invalid {field}")))
}

#[cfg(feature = "server")]
pub(crate) async fn require_admin() -> Result<crate::models::User, ServerFnError> {
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
    .ok_or_else(|| not_found("User not found"))?;

    if !user.is_admin() {
        return Err(forbidden("Admin access required"));
    }
    Ok(user)
}

#[cfg(feature = "server")]
pub(crate) async fn require_manager() -> Result<crate::models::User, ServerFnError> {
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
    .ok_or_else(|| not_found("User not found"))?;

    if !user.is_manager_or_above() {
        return Err(forbidden("Manager access required"));
    }
    Ok(user)
}

// ── Plugin event dispatch helpers ────────────────────────────────────────────
// Fire-and-forget: spawn dispatch so the core action is never blocked (FR-021).

#[cfg(feature = "server")]
pub(crate) async fn dispatch_time_entry_event(entry: &crate::models::TimeEntry, event_name: &str) {
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

#[cfg(feature = "server")]
pub(crate) fn time_entry_payload(
    e: &crate::models::TimeEntry,
) -> crate::plugin::event::TimeEntryPayload {
    crate::plugin::event::TimeEntryPayload {
        id: e.id,
        user_id: e.user_id,
        project_id: e.project_id,
        task_id: e.task_id,
        spent_date: e.spent_date,
        minutes: e.minutes,
        billable: e.billable,
        is_running: e.is_running,
        notes: e.notes.clone(),
        started_at: e.started_at,
    }
}

#[cfg(feature = "server")]
pub(crate) fn invoice_payload(
    inv: &crate::models::Invoice,
) -> crate::plugin::event::InvoicePayload {
    crate::plugin::event::InvoicePayload {
        id: inv.id,
        client_id: inv.client_id,
        invoice_number: inv.number.clone(),
        status: inv.status.to_string(),
        issue_date: inv.issued_on,
        due_date: inv.due_on,
        currency: inv.currency.clone(),
        total_cents: inv.total_cents,
    }
}

#[cfg(feature = "server")]
pub(crate) fn submission_payload(
    a: &crate::models::Approval,
    total_minutes: i32,
) -> crate::plugin::event::SubmissionPayload {
    crate::plugin::event::SubmissionPayload {
        id: a.id,
        user_id: a.user_id,
        week_start: a.period_start,
        status: a.state.to_string(),
        total_minutes,
    }
}

/// Sum of tracked minutes for a user across a period, for submission events.
#[cfg(feature = "server")]
pub(crate) async fn week_total_minutes(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
    start: chrono::NaiveDate,
    end: chrono::NaiveDate,
) -> Result<i32, ServerFnError> {
    sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(minutes), 0)::int as "total!"
           FROM time_entries
           WHERE user_id = $1 AND spent_date BETWEEN $2 AND $3"#,
        user_id,
        start as chrono::NaiveDate,
        end as chrono::NaiveDate,
    )
    .fetch_one(db)
    .await
    .map_err(server_err)
}

#[cfg(feature = "server")]
pub(crate) fn client_payload(c: &crate::models::Client) -> crate::plugin::event::ClientPayload {
    crate::plugin::event::ClientPayload {
        id: c.id,
        name: c.name.clone(),
        currency: c.currency.clone(),
        active: c.active,
    }
}

#[cfg(feature = "server")]
pub(crate) fn project_payload(p: &crate::models::Project) -> crate::plugin::event::ProjectPayload {
    crate::plugin::event::ProjectPayload {
        id: p.id,
        client_id: p.client_id,
        name: p.name.clone(),
        project_type: p.project_type.to_string(),
        budget_kind: p.budget_kind.to_string(),
        active: p.active,
    }
}

#[cfg(feature = "server")]
pub(crate) fn task_payload(t: &crate::models::Task) -> crate::plugin::event::TaskPayload {
    crate::plugin::event::TaskPayload {
        id: t.id,
        name: t.name.clone(),
        billable_default: t.billable_default,
        default_rate_cents: t.default_rate_cents,
        active: t.active,
    }
}

#[cfg(feature = "server")]
pub(crate) fn assignment_payload(
    a: &crate::models::Assignment,
) -> crate::plugin::event::AssignmentPayload {
    crate::plugin::event::AssignmentPayload {
        id: a.id,
        project_id: a.project_id,
        user_id: a.user_id,
        role: a.role.to_string(),
    }
}

#[cfg(feature = "server")]
pub(crate) fn user_payload(u: &crate::models::User) -> crate::plugin::event::UserPayload {
    crate::plugin::event::UserPayload {
        id: u.id,
        email: u.email.clone(),
        name: u.name.clone(),
        org_role: u.org_role.to_string(),
        method: None,
    }
}

/// Recompute an hours-budget project's consumption and, if it crossed a new
/// configured threshold band, dispatch the budget event once, then advance or
/// reset the stored band. Spawned fire-and-forget so it never blocks the write;
/// errors are logged, not propagated. Amount budgets are not evaluated yet —
/// they need FR-024 rate resolution.
#[cfg(feature = "server")]
pub(crate) async fn check_project_budget(
    state: &'static crate::state::AppState,
    project_id: uuid::Uuid,
) {
    let row = match sqlx::query!(
        r#"SELECT p.org_id, p.client_id, p.name, p.active, p.last_budget_alert_pct,
                  p.project_type as "project_type: horae_core::types::ProjectType",
                  p.budget_kind as "budget_kind: horae_core::types::BudgetKind",
                  p.budget_minutes,
                  o.budget_alert_pcts
           FROM projects p
           JOIN organizations o ON o.id = p.org_id
           WHERE p.id = $1"#,
        project_id,
    )
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(r)) => r,
        Ok(None) => return,
        Err(e) => {
            tracing::warn!("budget check: load project {project_id} failed: {e}");
            return;
        }
    };

    // Only hours budgets are evaluated for now (amount needs rate resolution).
    let budget = match (row.budget_kind, row.budget_minutes) {
        (horae_core::types::BudgetKind::Hours, Some(b)) if b > 0 => b,
        _ => return,
    };

    let consumed: i64 = match sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(COALESCE(rounded_minutes, minutes)), 0)::bigint as "c!"
           FROM time_entries WHERE project_id = $1"#,
        project_id,
    )
    .fetch_one(&state.db)
    .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("budget check: sum for {project_id} failed: {e}");
            return;
        }
    };

    let thresholds = row.budget_alert_pcts;
    let last = row.last_budget_alert_pct.unwrap_or(0);
    let current = horae_core::budget::current_band(consumed, budget, &thresholds);

    // Announce every band newly crossed since `last`. `100` is always a band, so
    // exceeding budget fires `project_over_budget` regardless of the configured
    // warning thresholds, and a single large jump reports each band it passed.
    for band in horae_core::budget::newly_crossed_bands(consumed, budget, &thresholds, last) {
        let payload = crate::plugin::event::BudgetThresholdPayload {
            project: crate::plugin::event::ProjectPayload {
                id: project_id,
                client_id: row.client_id,
                name: row.name.clone(),
                project_type: row.project_type.to_string(),
                budget_kind: row.budget_kind.to_string(),
                active: row.active,
            },
            threshold_pct: band,
            consumed_minutes: Some(consumed as i32),
            budget_minutes: Some(budget),
            consumed_cents: None,
            budget_amount_cents: None,
        };
        let occurred_at = chrono::Utc::now();
        let event = if band >= horae_core::budget::OVER_BUDGET_BAND {
            crate::plugin::AppEvent::ProjectOverBudget {
                occurred_at,
                org_id: row.org_id,
                budget: payload,
            }
        } else {
            crate::plugin::AppEvent::ProjectBudgetThresholdReached {
                occurred_at,
                org_id: row.org_id,
                budget: payload,
            }
        };
        state.plugins.dispatch(event);
    }

    // Advance (or reset) the stored band so each crossing fires at most once.
    if current != last
        && let Err(e) = sqlx::query!(
            "UPDATE projects SET last_budget_alert_pct = $2 WHERE id = $1",
            project_id,
            current,
        )
        .execute(&state.db)
        .await
    {
        tracing::warn!("budget check: store band for {project_id} failed: {e}");
    }
}

// ── Feature modules ──────────────────────────────────────────────────────────
// The #[server] endpoints grouped by feature; re-exported so call sites keep
// using `server_fns::<fn>` regardless of which submodule a function lives in.
mod approvals;
mod auth;
mod clients;
mod invoices;
mod organization;
mod projects;
mod reports;
mod time_entries;
mod users;

pub use approvals::*;
pub use auth::*;
pub use clients::*;
pub use invoices::*;
// Org-branding endpoints exist but no page consumes them yet.
#[allow(unused_imports)]
pub use organization::*;
pub use projects::*;
pub use reports::*;
pub use time_entries::*;
pub use users::*;
