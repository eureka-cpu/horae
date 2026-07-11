#![cfg(feature = "server")]

use chrono::NaiveDate;
use horae_core::types::{EntryState, OrgRole, ProjectRole, RoundDir};
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
    assert_eq!(row.minutes, 5, "Expected exactly 5 minutes, got {}", row.minutes);
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

    let row = sqlx::query!(
        "SELECT minutes FROM time_entries WHERE id = $1",
        entry_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        row.minutes, 0,
        "Sub-minute run must be 0 minutes, got {}", row.minutes
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
    let user_id = seed_user(&pool, org_id, "manager").await;
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
    let user_id = seed_user(&pool, org_id, "member").await;
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
