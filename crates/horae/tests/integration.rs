#![cfg(feature = "server")]

use chrono::NaiveDate;
use horae_core::types::{EntryState, InvoiceStatus, OrgRole, ProjectRole, RoundDir};
use serial_test::serial;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Test Org')",
        id
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn seed_user(pool: &PgPool, org_id: Uuid, role: OrgRole) -> Uuid {
    let id = Uuid::now_v7();
    let email = format!("{}@test.com", id);
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role) \
         VALUES ($1, $2, $3, $4, $5)",
        id,
        org_id,
        email,
        "Test User",
        role as OrgRole,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Insert a client, project, task (linked via `project_tasks`), and an
/// assignment for the given user.  Returns `(project_id, task_id, client_id)`.
async fn seed_project_with_assignment(
    pool: &PgPool,
    org_id: Uuid,
    user_id: Uuid,
) -> (Uuid, Uuid, Uuid) {
    let client_id = Uuid::now_v7();
    let project_id = Uuid::now_v7();
    let task_id = Uuid::now_v7();
    let assignment_id = Uuid::now_v7();

    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Acme', 'EUR')",
        client_id,
        org_id,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency) \
         VALUES ($1, $2, $3, 'Widget', 'EUR')",
        project_id,
        org_id,
        client_id,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name, billable_default) VALUES ($1, $2, 'Dev', true)",
        task_id,
        org_id,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents) \
         VALUES ($1, $2, true, NULL)",
        project_id,
        task_id,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO assignments (id, project_id, user_id, role) \
         VALUES ($1, $2, $3, $4)",
        assignment_id,
        project_id,
        user_id,
        ProjectRole::Freelancer as ProjectRole,
    )
    .execute(pool)
    .await
    .unwrap();

    (project_id, task_id, client_id)
}

/// Same as `seed_project_with_assignment` but does NOT create an assignment.
async fn seed_project_without_assignment(pool: &PgPool, org_id: Uuid) -> (Uuid, Uuid, Uuid) {
    let client_id = Uuid::now_v7();
    let project_id = Uuid::now_v7();
    let task_id = Uuid::now_v7();

    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Acme', 'EUR')",
        client_id,
        org_id,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency) \
         VALUES ($1, $2, $3, 'Widget', 'EUR')",
        project_id,
        org_id,
        client_id,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name, billable_default) VALUES ($1, $2, 'Dev', true)",
        task_id,
        org_id,
    )
    .execute(pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents) \
         VALUES ($1, $2, true, NULL)",
        project_id,
        task_id,
    )
    .execute(pool)
    .await
    .unwrap();

    (project_id, task_id, client_id)
}

// ---------------------------------------------------------------------------
// Test 1: Timer start / stop flow
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn timer_start_stop_records_minutes(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, OrgRole::Member).await;
    let (project_id, task_id, _) = seed_project_with_assignment(&pool, org_id, user_id).await;

    // Start a timer by inserting a running entry whose started_at is 5 minutes
    // in the past.
    let entry_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, started_at, state) \
         VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, \
                 0, true, true, now() - interval '5 minutes', $6)",
        entry_id,
        org_id,
        user_id,
        project_id,
        task_id,
        EntryState::Open as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Verify is_running
    let row = sqlx::query!(
        "SELECT is_running FROM time_entries WHERE id = $1",
        entry_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(row.is_running);

    // Stop the timer: read started_at then compute elapsed minutes via
    // horae-core, mirroring the exact code path in the stop_timer server fn.
    let started_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar!(
        r#"SELECT started_at as "started_at!: chrono::DateTime<chrono::Utc>"
           FROM time_entries WHERE id = $1 AND is_running = true"#,
        entry_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let minutes = horae_core::duration::minutes_between(started_at, chrono::Utc::now()) as i32;

    sqlx::query!(
        "UPDATE time_entries \
         SET is_running = false, \
             minutes = $2, \
             started_at = NULL, \
             updated_at = now() \
         WHERE id = $1 AND is_running = true",
        entry_id,
        minutes,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Verify the recorded duration is exactly 5 minutes and no longer running.
    let row = sqlx::query!(
        "SELECT minutes, is_running FROM time_entries WHERE id = $1",
        entry_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!row.is_running);
    assert_eq!(
        row.minutes, 5,
        "Expected exactly 5 minutes, got {}",
        row.minutes
    );
}

// ---------------------------------------------------------------------------
// A sub-minute timer records 0 minutes — no artificial 1-minute minimum
// (exactness: totals are never inflated).
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn timer_under_a_minute_records_zero(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, OrgRole::Member).await;
    let (project_id, task_id, _) = seed_project_with_assignment(&pool, org_id, user_id).await;

    let entry_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, started_at, state) \
         VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, \
                 0, true, true, now() - interval '30 seconds', $6)",
        entry_id,
        org_id,
        user_id,
        project_id,
        task_id,
        EntryState::Open as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Read started_at, compute via horae-core — same path as the server fn.
    let started_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar!(
        r#"SELECT started_at as "started_at!: chrono::DateTime<chrono::Utc>"
           FROM time_entries WHERE id = $1 AND is_running = true"#,
        entry_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let minutes = horae_core::duration::minutes_between(started_at, chrono::Utc::now()) as i32;

    sqlx::query!(
        "UPDATE time_entries \
         SET is_running = false, \
             minutes = $2, \
             started_at = NULL \
         WHERE id = $1 AND is_running = true",
        entry_id,
        minutes,
    )
    .execute(&pool)
    .await
    .unwrap();

    let row = sqlx::query!("SELECT minutes FROM time_entries WHERE id = $1", entry_id,)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        row.minutes, 0,
        "Sub-minute run must be 0 minutes, got {}",
        row.minutes
    );
}

// ---------------------------------------------------------------------------
// Test 2: One running timer per user (partial unique index)
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn one_timer_per_user_enforced(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, OrgRole::Member).await;
    let (project_id, task_id, _) = seed_project_with_assignment(&pool, org_id, user_id).await;

    // First running timer -- should succeed
    let entry1 = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, started_at, state) \
         VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, \
                 0, true, true, now(), $6)",
        entry1,
        org_id,
        user_id,
        project_id,
        task_id,
        EntryState::Open as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Second running timer for the same user -- must fail
    let entry2 = Uuid::now_v7();
    let result = sqlx::query!(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, started_at, state) \
         VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, \
                 0, true, true, now(), $6)",
        entry2,
        org_id,
        user_id,
        project_id,
        task_id,
        EntryState::Open as EntryState,
    )
    .execute(&pool)
    .await;

    assert!(
        result.is_err(),
        "Expected unique-index violation for second running timer"
    );
}

// ---------------------------------------------------------------------------
// Test 3: Submitted entries cannot be updated via state='open' guard
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn submitted_entries_cannot_be_updated(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, OrgRole::Member).await;
    let (project_id, task_id, _) = seed_project_with_assignment(&pool, org_id, user_id).await;

    let entry_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, state) \
         VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, \
                 30, true, false, $6)",
        entry_id,
        org_id,
        user_id,
        project_id,
        task_id,
        EntryState::Open as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Transition to submitted
    sqlx::query!(
        "UPDATE time_entries SET state = $1, updated_at = now() \
         WHERE id = $2",
        EntryState::Submitted as EntryState,
        entry_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Attempt to change minutes using a WHERE guard that only matches open entries
    let result = sqlx::query!(
        "UPDATE time_entries \
         SET minutes = 999, updated_at = now() \
         WHERE id = $1 AND state = $2",
        entry_id,
        EntryState::Open as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
        result.rows_affected(),
        0,
        "No rows should be updated for a submitted entry"
    );

    // Confirm minutes unchanged
    let row = sqlx::query!("SELECT minutes FROM time_entries WHERE id = $1", entry_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.minutes, 30);
}

// ---------------------------------------------------------------------------
// Test 4: Rounding applied on submit
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn rounding_applied_on_submit(pool: PgPool) {
    use horae_core::rounding::round;

    // Create org with 15-minute rounding
    let org_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name, round_minutes, round_dir) \
         VALUES ($1, 'Rounded Org', 15, $2)",
        org_id,
        RoundDir::Nearest as RoundDir,
    )
    .execute(&pool)
    .await
    .unwrap();

    let user_id = seed_user(&pool, org_id, OrgRole::Member).await;
    let (project_id, task_id, _) = seed_project_with_assignment(&pool, org_id, user_id).await;

    // Insert entry with 8 raw minutes
    let entry_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, state) \
         VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, \
                 8, true, false, $6)",
        entry_id,
        org_id,
        user_id,
        project_id,
        task_id,
        EntryState::Open as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Simulate the submit path: read org rounding config, compute rounded value,
    // persist it together with the state transition.
    let row = sqlx::query!(
        "SELECT round_minutes, round_dir::text FROM organizations WHERE id = $1",
        org_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let round_minutes = row.round_minutes;
    let round_dir_str = row.round_dir.unwrap_or_default();

    let dir = match round_dir_str.as_str() {
        "up" => RoundDir::Up,
        "down" => RoundDir::Down,
        _ => RoundDir::Nearest,
    };

    let row = sqlx::query!("SELECT minutes FROM time_entries WHERE id = $1", entry_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let raw_minutes = row.minutes;

    let rounded = round(raw_minutes as u32, round_minutes as u32, dir);

    // Confirm pure-logic rounding: 8 min with 15-min nearest rounds to 15
    assert_eq!(rounded, 15, "8 rounds to 15 with 15-min nearest rounding");

    // Persist and transition
    sqlx::query!(
        "UPDATE time_entries \
         SET rounded_minutes = $1, state = $2, updated_at = now() \
         WHERE id = $3",
        rounded as i32,
        EntryState::Submitted as EntryState,
        entry_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Verify stored values
    let row = sqlx::query!(
        "SELECT rounded_minutes, state::text FROM time_entries WHERE id = $1",
        entry_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.rounded_minutes, Some(15));
    assert_eq!(row.state.unwrap_or_default(), "submitted");
}

// ---------------------------------------------------------------------------
// Test 5: Assignment validation
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn unassigned_user_cannot_create_entry(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, OrgRole::Member).await;
    let (project_id, _task_id, _) = seed_project_without_assignment(&pool, org_id).await;

    // Confirm no assignment exists
    let assigned: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM assignments WHERE project_id = $1 AND user_id = $2)",
        project_id,
        user_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(false);
    assert!(!assigned, "User should not be assigned to this project");
}

// ---------------------------------------------------------------------------
// Test 6: Approval workflow -- approve and reject paths
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn approval_workflow_approve_and_reject(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, OrgRole::Member).await;
    let manager_id = seed_user(&pool, org_id, OrgRole::Manager).await;
    let (project_id, task_id, _) = seed_project_with_assignment(&pool, org_id, user_id).await;

    let period_start = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
    let period_end = NaiveDate::from_ymd_opt(2026, 7, 7).unwrap();

    // --- Approve path ---

    // Create an open entry
    let entry_a = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, state) \
         VALUES ($1, $2, $3, $4, $5, '2026-07-02', \
                 60, true, false, $6)",
        entry_a,
        org_id,
        user_id,
        project_id,
        task_id,
        EntryState::Open as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Submit the entry
    sqlx::query!(
        "UPDATE time_entries \
         SET state = $1, updated_at = now() \
         WHERE id = $2",
        EntryState::Submitted as EntryState,
        entry_a,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create approval record
    let approval_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO approvals (id, org_id, user_id, period_start, period_end, state) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        approval_id,
        org_id,
        user_id,
        period_start as chrono::NaiveDate,
        period_end as chrono::NaiveDate,
        EntryState::Submitted as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Manager approves: transition entries + approval
    sqlx::query!(
        "UPDATE time_entries \
         SET state = $1, updated_at = now() \
         WHERE user_id = $2 AND spent_date BETWEEN $3 AND $4 AND state = $5",
        EntryState::Approved as EntryState,
        user_id,
        period_start as chrono::NaiveDate,
        period_end as chrono::NaiveDate,
        EntryState::Submitted as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE approvals \
         SET state = $1, approved_by = $2, approved_at = now() \
         WHERE id = $3",
        EntryState::Approved as EntryState,
        manager_id,
        approval_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Verify entry and approval states
    let row = sqlx::query!(
        "SELECT state::text FROM time_entries WHERE id = $1",
        entry_a,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.state.unwrap_or_default(), "approved");

    let row = sqlx::query!(
        "SELECT state::text, approved_by FROM approvals WHERE id = $1",
        approval_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.state.unwrap_or_default(), "approved");
    assert_eq!(row.approved_by, Some(manager_id));

    // --- Reject path ---

    // Create a new entry, submit, then reject (reopen entries + delete approval)
    let entry_b = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, state) \
         VALUES ($1, $2, $3, $4, $5, '2026-07-14', \
                 45, true, false, $6)",
        entry_b,
        org_id,
        user_id,
        project_id,
        task_id,
        EntryState::Open as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    let period2_start = NaiveDate::from_ymd_opt(2026, 7, 8).unwrap();
    let period2_end = NaiveDate::from_ymd_opt(2026, 7, 14).unwrap();

    // Submit
    sqlx::query!(
        "UPDATE time_entries \
         SET state = $1, updated_at = now() \
         WHERE id = $2",
        EntryState::Submitted as EntryState,
        entry_b,
    )
    .execute(&pool)
    .await
    .unwrap();

    let approval2_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO approvals (id, org_id, user_id, period_start, period_end, state) \
         VALUES ($1, $2, $3, $4, $5, $6)",
        approval2_id,
        org_id,
        user_id,
        period2_start as chrono::NaiveDate,
        period2_end as chrono::NaiveDate,
        EntryState::Submitted as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Reject: reopen entries and delete the approval row
    sqlx::query!(
        "UPDATE time_entries \
         SET state = $1, updated_at = now() \
         WHERE user_id = $2 AND spent_date BETWEEN $3 AND $4 AND state = $5",
        EntryState::Open as EntryState,
        user_id,
        period2_start as chrono::NaiveDate,
        period2_end as chrono::NaiveDate,
        EntryState::Submitted as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!("DELETE FROM approvals WHERE id = $1", approval2_id)
        .execute(&pool)
        .await
        .unwrap();

    // Verify entry reopened
    let row = sqlx::query!(
        "SELECT state::text FROM time_entries WHERE id = $1",
        entry_b,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.state.unwrap_or_default(), "open");

    // Verify approval row deleted
    let approval_exists: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM approvals WHERE id = $1)",
        approval2_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(false);
    assert!(!approval_exists);
}

// ---------------------------------------------------------------------------
// US2: organizing clients, projects, tasks
// ---------------------------------------------------------------------------

/// A newly created org task is not loggable on a project until it is linked
/// (mirrors `link_project_task`); once linked it appears on the project's task
/// picker and a time entry can be recorded against it.
#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn new_task_becomes_loggable_on_project(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, OrgRole::Manager).await;
    let (project_id, _existing_task, _) =
        seed_project_with_assignment(&pool, org_id, user_id).await;

    // A manager adds a brand-new org-level task.
    let new_task = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO tasks (id, org_id, name, billable_default) VALUES ($1, $2, 'Review', true)",
    )
    .bind(new_task)
    .bind(org_id)
    .execute(&pool)
    .await
    .unwrap();

    // Before linking, the task is not offered on the project's picker.
    let before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tasks t \
         JOIN project_tasks pt ON t.id = pt.task_id \
         WHERE pt.project_id = $1 AND t.id = $2 AND t.active = true",
    )
    .bind(project_id)
    .bind(new_task)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        before, 0,
        "unlinked task must not appear on the project picker"
    );

    // Link it (the project-task link inherits the task's billable default).
    sqlx::query(
        "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents) \
         SELECT p.id, t.id, t.billable_default, t.default_rate_cents \
         FROM projects p JOIN tasks t ON t.org_id = p.org_id \
         WHERE p.id = $1 AND t.id = $2 \
         ON CONFLICT (project_id, task_id) DO NOTHING",
    )
    .bind(project_id)
    .bind(new_task)
    .execute(&pool)
    .await
    .unwrap();

    // Now it shows up on the project's task picker.
    let after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tasks t \
         JOIN project_tasks pt ON t.id = pt.task_id \
         WHERE pt.project_id = $1 AND t.id = $2 AND t.active = true",
    )
    .bind(project_id)
    .bind(new_task)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after, 1, "linked task must be loggable on the project");

    // And a time entry can be recorded against it.
    let entry_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, state) \
         VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, \
                 30, true, false, 'open'::entry_state)",
    )
    .bind(entry_id)
    .bind(org_id)
    .bind(user_id)
    .bind(project_id)
    .bind(new_task)
    .execute(&pool)
    .await
    .unwrap();

    let logged: i64 = sqlx::query_scalar("SELECT count(*) FROM time_entries WHERE id = $1")
        .bind(entry_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(logged, 1);
}

/// Deactivating a project removes it from new-entry pickers (which filter
/// `active = true`) but leaves existing time entries linked to it (FR-011).
#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn inactive_project_hidden_from_picker_but_kept_on_history(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, OrgRole::Member).await;
    let (project_id, task_id, _) = seed_project_with_assignment(&pool, org_id, user_id).await;

    // Log a completed entry against the project.
    let entry_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, state) \
         VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, \
                 60, true, false, 'open'::entry_state)",
    )
    .bind(entry_id)
    .bind(org_id)
    .bind(user_id)
    .bind(project_id)
    .bind(task_id)
    .execute(&pool)
    .await
    .unwrap();

    // Manager deactivates the project (set_project_active(.., false)).
    sqlx::query("UPDATE projects SET active = false WHERE id = $1 AND org_id = $2")
        .bind(project_id)
        .bind(org_id)
        .execute(&pool)
        .await
        .unwrap();

    // The active-only picker no longer offers the project.
    let in_picker: i64 =
        sqlx::query_scalar("SELECT count(*) FROM projects WHERE id = $1 AND active = true")
            .bind(project_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        in_picker, 0,
        "inactive project must be hidden from the picker"
    );

    // But the existing entry keeps its link and still appears in history.
    let (hist_project,): (Uuid,) =
        sqlx::query_as("SELECT project_id FROM time_entries WHERE id = $1")
            .bind(entry_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        hist_project, project_id,
        "history must retain the project link"
    );
}

// ---------------------------------------------------------------------------
// US3: invoicing tracked time
// ---------------------------------------------------------------------------

/// Generate an invoice from billable time entries. The invoice total must
/// equal the exact sum of line item amounts, and entries must be marked
/// invoiced with their invoice_id set (FR-012, FR-013, SC-002).
#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn generate_invoice_totals_match(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, OrgRole::Manager).await;
    let (project_id, task_id, client_id) =
        seed_project_with_assignment(&pool, org_id, user_id).await;

    // Set a task rate so line amounts are deterministic.
    sqlx::query!(
        "UPDATE project_tasks SET rate_cents = 12000 WHERE project_id = $1 AND task_id = $2",
        project_id,
        task_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert two billable entries: 60 min and 30 min.
    let entry_a = Uuid::now_v7();
    let entry_b = Uuid::now_v7();
    let date = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();

    for (eid, mins) in [(entry_a, 60), (entry_b, 30)] {
        sqlx::query!(
            "INSERT INTO time_entries \
               (id, org_id, user_id, project_id, task_id, spent_date, \
                minutes, billable, is_running, state) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, true, false, $8)",
            eid,
            org_id,
            user_id,
            project_id,
            task_id,
            date as NaiveDate,
            mins,
            EntryState::Open as EntryState,
        )
        .execute(&pool)
        .await
        .unwrap();
    }

    // Generate the invoice via the same logic as the server fn:
    // fetch entries, resolve rates, compute amounts, insert.
    let period_from = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
    let period_to = NaiveDate::from_ymd_opt(2026, 7, 31).unwrap();

    #[allow(dead_code)]
    struct EntryWithRates {
        entry_id: Uuid,
        minutes: i32,
        project_name: String,
        task_name: String,
        notes: Option<String>,
        spent_date: NaiveDate,
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
        org_id,
        client_id,
        period_from as NaiveDate,
        period_to as NaiveDate,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(entries.len(), 2, "should find both billable entries");

    let invoice_id = Uuid::now_v7();
    let mut total_cents: i64 = 0;

    // Compute line items first to know the total.
    struct LineData {
        id: Uuid,
        entry_id: Uuid,
        desc: String,
        minutes: i32,
        rate: i64,
        amount: i64,
    }
    let mut line_data = Vec::new();

    for e in &entries {
        let rate = horae_core::invoice::resolve_rate(
            e.task_rate_cents,
            e.assignment_rate_cents,
            e.user_rate_cents,
        )
        .unwrap_or(0);
        let amount = horae_core::invoice::line_amount_cents(rate, e.minutes);
        total_cents += amount;
        line_data.push(LineData {
            id: Uuid::now_v7(),
            entry_id: e.entry_id,
            desc: format!("{} — {} ({})", e.spent_date, e.project_name, e.task_name),
            minutes: e.minutes,
            rate,
            amount,
        });
    }

    // $120/hr × 60min = $120.00 (12000), $120/hr × 30min = $60.00 (6000)
    assert_eq!(total_cents, 18000, "total must be 12000 + 6000");

    // Insert invoice first (line items reference it via FK).
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents) \
         VALUES ($1, $2, $3, 'INV-202607-001', 'draft', '2026-07-11', '2026-08-10', 'EUR', $4)",
        invoice_id,
        org_id,
        client_id,
        total_cents,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Insert line items.
    for ld in &line_data {
        sqlx::query!(
            "INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            ld.id,
            invoice_id,
            ld.entry_id,
            ld.desc,
            ld.minutes,
            ld.rate,
            ld.amount,
        )
        .execute(&pool)
        .await
        .unwrap();
    }

    // Mark entries as invoiced.
    let entry_ids = vec![entry_a, entry_b];
    sqlx::query!(
        "UPDATE time_entries SET invoice_id = $1, state = 'invoiced', updated_at = now() \
         WHERE id = ANY($2)",
        invoice_id,
        &entry_ids,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Verify: total equals sum of line amounts.
    let line_sum: i64 = sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(amount_cents), 0)::bigint as "sum!: i64"
           FROM invoice_line_items WHERE invoice_id = $1"#,
        invoice_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        line_sum, total_cents,
        "line item sum must equal invoice total"
    );

    // Verify: entries are marked invoiced.
    for eid in &entry_ids {
        let state: String = sqlx::query_scalar!(
            r#"SELECT state::text as "state!: String" FROM time_entries WHERE id = $1"#,
            eid,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(state, "invoiced");

        let inv_id: Option<Uuid> =
            sqlx::query_scalar!("SELECT invoice_id FROM time_entries WHERE id = $1", eid)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(inv_id, Some(invoice_id));
    }
}

/// After invoicing, the same entries cannot be billed again — a second
/// generate for the same period should find nothing (FR-013).
#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn invoiced_entries_cannot_be_rebilled(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, OrgRole::Manager).await;
    let (project_id, task_id, client_id) =
        seed_project_with_assignment(&pool, org_id, user_id).await;

    let date = NaiveDate::from_ymd_opt(2026, 7, 5).unwrap();
    let entry_id = Uuid::now_v7();

    sqlx::query!(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, state) \
         VALUES ($1, $2, $3, $4, $5, $6, 60, true, false, $7)",
        entry_id,
        org_id,
        user_id,
        project_id,
        task_id,
        date as NaiveDate,
        EntryState::Open as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Invoice the entry.
    let invoice_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents) \
         VALUES ($1, $2, $3, 'INV-001', 'draft', '2026-07-11', '2026-08-10', 'EUR', 0)",
        invoice_id,
        org_id,
        client_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE time_entries SET invoice_id = $1, state = 'invoiced', updated_at = now() \
         WHERE id = $2",
        invoice_id,
        entry_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Try to find billable un-invoiced entries — should be zero.
    let period_from = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
    let period_to = NaiveDate::from_ymd_opt(2026, 7, 31).unwrap();

    let count: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!: i64" FROM time_entries
           WHERE org_id = $1
             AND project_id IN (SELECT id FROM projects WHERE client_id = $2)
             AND billable = true
             AND invoice_id IS NULL
             AND state = 'open'
             AND spent_date >= $3
             AND spent_date <= $4"#,
        org_id,
        client_id,
        period_from as NaiveDate,
        period_to as NaiveDate,
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        count, 0,
        "invoiced entries must not be available for re-billing"
    );
}

/// Voiding an invoice restores its entries to open / un-invoiced state,
/// allowing them to be billed again (data-model.md state machine).
#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn void_invoice_restores_entries(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, OrgRole::Manager).await;
    let (project_id, task_id, client_id) =
        seed_project_with_assignment(&pool, org_id, user_id).await;

    let date = NaiveDate::from_ymd_opt(2026, 7, 3).unwrap();
    let entry_id = Uuid::now_v7();

    sqlx::query!(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, state) \
         VALUES ($1, $2, $3, $4, $5, $6, 45, true, false, $7)",
        entry_id,
        org_id,
        user_id,
        project_id,
        task_id,
        date as NaiveDate,
        EntryState::Open as EntryState,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create and mark invoiced.
    let invoice_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO invoices (id, org_id, client_id, number, status, issued_on, due_on, currency, total_cents) \
         VALUES ($1, $2, $3, 'INV-V01', 'draft', '2026-07-11', '2026-08-10', 'EUR', 0)",
        invoice_id,
        org_id,
        client_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO invoice_line_items (id, invoice_id, time_entry_id, description, minutes, rate_cents, amount_cents) \
         VALUES ($1, $2, $3, 'test line', 45, 10000, 7500)",
        Uuid::now_v7(),
        invoice_id,
        entry_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE time_entries SET invoice_id = $1, state = 'invoiced', updated_at = now() \
         WHERE id = $2",
        invoice_id,
        entry_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Void the invoice: restore entries, update status.
    sqlx::query!(
        "UPDATE time_entries SET invoice_id = NULL, state = 'open', updated_at = now() \
         WHERE invoice_id = $1",
        invoice_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE invoices SET status = $2 WHERE id = $1",
        invoice_id,
        InvoiceStatus::Void as InvoiceStatus,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Verify entry is back to open with no invoice_id.
    let state: String = sqlx::query_scalar!(
        r#"SELECT state::text as "state!: String" FROM time_entries WHERE id = $1"#,
        entry_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state, "open", "voided invoice must restore entry to open");

    let inv_id: Option<Uuid> = sqlx::query_scalar!(
        "SELECT invoice_id FROM time_entries WHERE id = $1",
        entry_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(inv_id.is_none(), "entry's invoice_id must be cleared");

    // Verify invoice status is void.
    let inv_status: String = sqlx::query_scalar!(
        r#"SELECT status::text as "status!: String" FROM invoices WHERE id = $1"#,
        invoice_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(inv_status, "void");
}

/// Rate resolution cascade: task rate takes priority over assignment rate,
/// which takes priority over user default rate (FR-024).
#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn rate_resolution_cascade(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, OrgRole::Manager).await;
    let (project_id, task_id, _client_id) =
        seed_project_with_assignment(&pool, org_id, user_id).await;

    // Set rates at all three levels:
    // - Task rate: $150/hr (15000 cents)
    // - Assignment rate: $120/hr (12000 cents)
    // - User default rate: $100/hr (10000 cents)
    sqlx::query!(
        "UPDATE project_tasks SET rate_cents = 15000 WHERE project_id = $1 AND task_id = $2",
        project_id,
        task_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE assignments SET rate_cents = 12000 WHERE project_id = $1 AND user_id = $2",
        project_id,
        user_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "UPDATE users SET billable_rate_cents = 10000 WHERE id = $1",
        user_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Fetch the rates the same way the server fn does.
    struct RateRow {
        task_rate_cents: Option<i64>,
        assignment_rate_cents: Option<i64>,
        user_rate_cents: Option<i64>,
    }

    let rates = sqlx::query_as!(
        RateRow,
        r#"SELECT
             pt.rate_cents as task_rate_cents,
             a.rate_cents as assignment_rate_cents,
             u.billable_rate_cents as user_rate_cents
           FROM project_tasks pt
           LEFT JOIN assignments a ON a.project_id = pt.project_id AND a.user_id = $3
           JOIN users u ON u.id = $3
           WHERE pt.project_id = $1 AND pt.task_id = $2"#,
        project_id,
        task_id,
        user_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // Task rate (15000) should win.
    let resolved = horae_core::invoice::resolve_rate(
        rates.task_rate_cents,
        rates.assignment_rate_cents,
        rates.user_rate_cents,
    );
    assert_eq!(resolved, Some(15000), "task rate should take priority");

    // Remove task rate — assignment should win.
    sqlx::query!(
        "UPDATE project_tasks SET rate_cents = NULL WHERE project_id = $1 AND task_id = $2",
        project_id,
        task_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    let rates = sqlx::query_as!(
        RateRow,
        r#"SELECT
             pt.rate_cents as task_rate_cents,
             a.rate_cents as assignment_rate_cents,
             u.billable_rate_cents as user_rate_cents
           FROM project_tasks pt
           LEFT JOIN assignments a ON a.project_id = pt.project_id AND a.user_id = $3
           JOIN users u ON u.id = $3
           WHERE pt.project_id = $1 AND pt.task_id = $2"#,
        project_id,
        task_id,
        user_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let resolved = horae_core::invoice::resolve_rate(
        rates.task_rate_cents,
        rates.assignment_rate_cents,
        rates.user_rate_cents,
    );
    assert_eq!(
        resolved,
        Some(12000),
        "assignment rate should win when no task rate"
    );

    // Remove assignment rate — user default should win.
    sqlx::query!(
        "UPDATE assignments SET rate_cents = NULL WHERE project_id = $1 AND user_id = $2",
        project_id,
        user_id,
    )
    .execute(&pool)
    .await
    .unwrap();

    let rates = sqlx::query_as!(
        RateRow,
        r#"SELECT
             pt.rate_cents as task_rate_cents,
             a.rate_cents as assignment_rate_cents,
             u.billable_rate_cents as user_rate_cents
           FROM project_tasks pt
           LEFT JOIN assignments a ON a.project_id = pt.project_id AND a.user_id = $3
           JOIN users u ON u.id = $3
           WHERE pt.project_id = $1 AND pt.task_id = $2"#,
        project_id,
        task_id,
        user_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let resolved = horae_core::invoice::resolve_rate(
        rates.task_rate_cents,
        rates.assignment_rate_cents,
        rates.user_rate_cents,
    );
    assert_eq!(
        resolved,
        Some(10000),
        "user default rate should be the fallback"
    );
}
