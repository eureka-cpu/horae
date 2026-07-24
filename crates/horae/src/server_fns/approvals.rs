//! Approval server functions.

use super::*;

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
        return Err(not_found("No open entries found for this week"));
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

    let total_minutes = week_total_minutes(&state.db, user_id, ws, we).await?;
    state
        .plugins
        .dispatch(crate::plugin::AppEvent::TimesheetSubmitted {
            occurred_at: chrono::Utc::now(),
            org_id,
            submission: submission_payload(&approval, total_minutes),
        });

    Ok(approval)
}

/// List approvals, optionally filtered by state. Requires manager role.
#[server]
pub async fn list_approvals(status: Option<String>) -> Result<Vec<ApprovalSummary>, ServerFnError> {
    let _manager = require_manager().await?;
    let state = crate::state::global_state().await;

    let state_filter: Option<EntryState> = status
        .map(|s| {
            s.parse::<EntryState>()
                .map_err(|_| server_err("Invalid status"))
        })
        .transpose()?;

    // Hours are aggregated per row from the user's entries in the approval's
    // period (actual `minutes`, split by `billable`) via a lateral join, so the
    // whole table comes back in one query rather than a lookup per approval.
    let rows = sqlx::query!(
        r#"SELECT a.id, a.org_id, a.user_id,
                a.period_start as "period_start: chrono::NaiveDate",
                a.period_end as "period_end: chrono::NaiveDate",
                a.state as "state: EntryState",
                a.submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                a.approved_by,
                a.approved_at as "approved_at: chrono::DateTime<chrono::Utc>",
                COALESCE(t.total_minutes, 0) as "total_minutes!",
                COALESCE(t.billable_minutes, 0) as "billable_minutes!"
         FROM approvals a
         LEFT JOIN LATERAL (
             SELECT (SUM(minutes))::bigint as total_minutes,
                    (SUM(minutes) FILTER (WHERE billable))::bigint as billable_minutes
             FROM time_entries te
             WHERE te.user_id = a.user_id
               AND te.spent_date BETWEEN a.period_start AND a.period_end
         ) t ON true
         WHERE ($1::entry_state IS NULL OR a.state = $1)
         ORDER BY a.period_start DESC"#,
        state_filter as Option<EntryState>,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(rows
        .into_iter()
        .map(|r| ApprovalSummary {
            approval: Approval {
                id: r.id,
                org_id: r.org_id,
                user_id: r.user_id,
                period_start: r.period_start,
                period_end: r.period_end,
                state: r.state,
                submitted_at: r.submitted_at,
                approved_by: r.approved_by,
                approved_at: r.approved_at,
            },
            total_minutes: r.total_minutes,
            billable_minutes: r.billable_minutes,
        })
        .collect())
}

/// Approve every submitted approval in `ids` within one transaction: flip each
/// row Submitted→Approved, transition its period's submitted entries, and return
/// the rows actually approved (ids not in 'submitted' are skipped). Shared by the
/// single- and bulk-approve server functions so the transition lives in one place.
#[cfg(feature = "server")]
async fn approve_ids(manager: &User, ids: &[uuid::Uuid]) -> Result<Vec<Approval>, ServerFnError> {
    let state = crate::state::global_state().await;
    let mut tx = state.db.begin().await.map_err(server_err)?;

    let approvals = sqlx::query_as!(
        Approval,
        r#"UPDATE approvals
             SET state = $2, approved_by = $3, approved_at = now()
           WHERE id = ANY($1) AND state = $4
        RETURNING id, org_id, user_id,
                  period_start as "period_start: chrono::NaiveDate",
                  period_end as "period_end: chrono::NaiveDate",
                  state as "state: EntryState",
                  submitted_at as "submitted_at: chrono::DateTime<chrono::Utc>",
                  approved_by,
                  approved_at as "approved_at: chrono::DateTime<chrono::Utc>""#,
        ids,
        EntryState::Approved as EntryState,
        manager.id,
        EntryState::Submitted as EntryState,
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(server_err)?;

    // Transition the submitted entries of every approved period to approved.
    let approved_ids: Vec<uuid::Uuid> = approvals.iter().map(|a| a.id).collect();
    sqlx::query!(
        r#"UPDATE time_entries te
              SET state = $2, updated_at = now()
             FROM approvals a
            WHERE a.id = ANY($1)
              AND te.user_id = a.user_id
              AND te.spent_date BETWEEN a.period_start AND a.period_end
              AND te.state = $3"#,
        &approved_ids,
        EntryState::Approved as EntryState,
        EntryState::Submitted as EntryState,
    )
    .execute(&mut *tx)
    .await
    .map_err(server_err)?;

    tx.commit().await.map_err(server_err)?;

    // Announce each approval (FR-019) once the transition is durably committed.
    for a in &approvals {
        let total_minutes =
            week_total_minutes(&state.db, a.user_id, a.period_start, a.period_end).await?;
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::SubmissionApproved {
                occurred_at: chrono::Utc::now(),
                org_id: a.org_id,
                submission: submission_payload(a, total_minutes),
            });
    }

    Ok(approvals)
}

/// Ensure the caller may approve, mapping an insufficient role to a 403.
#[cfg(feature = "server")]
fn ensure_can_approve(manager: &User) -> Result<(), ServerFnError> {
    if horae_core::state::can_transition(
        EntryState::Submitted,
        EntryState::Approved,
        manager.org_role,
    ) {
        Ok(())
    } else {
        Err(forbidden("Insufficient role to approve submissions"))
    }
}

/// Approve a single submitted week. Requires manager role.
#[server]
pub async fn approve_submission(approval_id: String) -> Result<Approval, ServerFnError> {
    let manager = require_manager().await?;
    ensure_can_approve(&manager)?;
    let id = parse_uuid(&approval_id, "approval_id")?;

    approve_ids(&manager, &[id])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| not_found("Approval not found or not in 'submitted' state"))
}

/// Approve several submitted weeks at once (the "approve visible" action).
/// Returns the number actually approved; ids not in 'submitted' are skipped.
#[server]
pub async fn approve_submissions(approval_ids: Vec<String>) -> Result<usize, ServerFnError> {
    let manager = require_manager().await?;
    ensure_can_approve(&manager)?;
    let ids = approval_ids
        .iter()
        .map(|s| parse_uuid(s, "approval_id"))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(approve_ids(&manager, &ids).await?.len())
}

/// Reject a submitted week. Requires manager role.
/// Reopens the time entries and deletes the approval row.
#[server]
pub async fn reject_submission(approval_id: String) -> Result<(), ServerFnError> {
    let manager = require_manager().await?;

    if !horae_core::state::can_transition(EntryState::Submitted, EntryState::Open, manager.org_role)
    {
        return Err(forbidden("Insufficient role to reject submissions"));
    }

    let state = crate::state::global_state().await;
    let approval_id = parse_uuid(&approval_id, "approval_id")?;

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
    .ok_or_else(|| not_found("Approval not found"))?;

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

    let total_minutes = week_total_minutes(
        &state.db,
        approval.user_id,
        approval.period_start,
        approval.period_end,
    )
    .await?;
    state
        .plugins
        .dispatch(crate::plugin::AppEvent::SubmissionRejected {
            occurred_at: chrono::Utc::now(),
            org_id: approval.org_id,
            submission: submission_payload(&approval, total_minutes),
        });

    Ok(())
}
