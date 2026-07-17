//! Time-entry server functions.

use super::*;

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
    let project_id = parse_uuid(&project_id, "project_id")?;
    let task_id = parse_uuid(&task_id, "task_id")?;

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
        return Err(conflict("A timer is already running. Stop it first."));
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
    let entry_id = parse_uuid(&entry_id, "entry_id")?;

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
    .ok_or_else(|| not_found("No running timer found for this entry"))?;

    let minutes = horae_core::duration::minutes_between(started_at, chrono::Utc::now()) as i32;

    let entry = sqlx::query_as!(
        TimeEntry,
        r#"UPDATE time_entries
         SET is_running = false,
             minutes = $3,
             started_at = NULL,
             notified_long_running_at = NULL,
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
    .ok_or_else(|| not_found("No running timer found for this entry"))?;

    dispatch_time_entry_event(&entry, "time_entry_stopped").await;
    tokio::spawn(check_project_budget(state, entry.project_id));
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
    let project_id = parse_uuid(&project_id, "project_id")?;
    let task_id = parse_uuid(&task_id, "task_id")?;
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
            return Err(forbidden("You are not assigned to this project"));
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
    tokio::spawn(check_project_budget(state, entry.project_id));
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
    let entry_id = parse_uuid(&entry_id, "entry_id")?;

    // Read current values first so a no-op update emits no event (FR-012).
    let before = sqlx::query!(
        r#"SELECT minutes, notes, billable FROM time_entries
           WHERE id = $1 AND user_id = $2 AND state = $3"#,
        entry_id,
        user_id,
        EntryState::Open as EntryState,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?;

    let entry = sqlx::query_as!(
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
    .ok_or_else(|| conflict("Entry not found or is locked (not in 'open' state)"))?;

    let changed = before.is_none_or(|b| {
        b.minutes != minutes || b.notes.as_deref() != notes.as_deref() || b.billable != billable
    });
    if changed {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::TimeEntryUpdated {
                occurred_at: chrono::Utc::now(),
                org_id: entry.org_id,
                time_entry: time_entry_payload(&entry),
            });
    }

    tokio::spawn(check_project_budget(state, entry.project_id));
    Ok(entry)
}

/// Delete a time entry. Only allowed while the entry state is 'open'.
#[server]
pub async fn delete_time_entry(entry_id: String) -> Result<(), ServerFnError> {
    let user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let entry_id = parse_uuid(&entry_id, "entry_id")?;

    // Delete and capture the row in one statement so the "only open entries"
    // guard holds atomically (no TOCTOU) and the event carries the removed
    // entry's details.
    let entry = sqlx::query_as!(
        TimeEntry,
        r#"DELETE FROM time_entries
           WHERE id = $1 AND user_id = $2 AND state = $3
           RETURNING id, org_id, user_id, project_id, task_id,
                     spent_date as "spent_date: chrono::NaiveDate",
                     minutes, rounded_minutes, notes, billable, is_running,
                     started_at as "started_at: chrono::DateTime<chrono::Utc>",
                     state as "state: EntryState", invoice_id,
                     created_at as "created_at: chrono::DateTime<chrono::Utc>",
                     updated_at as "updated_at: chrono::DateTime<chrono::Utc>""#,
        entry_id,
        user_id,
        EntryState::Open as EntryState,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?
    .ok_or_else(|| conflict("Entry not found or is locked (not in 'open' state)"))?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::TimeEntryDeleted {
            occurred_at: chrono::Utc::now(),
            org_id: entry.org_id,
            time_entry: time_entry_payload(&entry),
        });

    tokio::spawn(check_project_budget(state, entry.project_id));
    Ok(())
}
