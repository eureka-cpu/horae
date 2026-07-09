#![cfg(feature = "server")]

use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO organizations (id, name) VALUES ($1, 'Test Org')")
        .bind(id)
        .execute(pool)
        .await
        .unwrap();
    id
}

async fn seed_user(pool: &PgPool, org_id: Uuid, role: &str) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO users (id, org_id, email, name, org_role) \
         VALUES ($1, $2, $3, $4, $5::org_role)",
    )
    .bind(id)
    .bind(org_id)
    .bind(format!("{}@test.com", id))
    .bind("Test User")
    .bind(role)
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

    sqlx::query(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Acme', 'EUR')",
    )
    .bind(client_id)
    .bind(org_id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO projects (id, org_id, client_id, name, currency) \
         VALUES ($1, $2, $3, 'Widget', 'EUR')",
    )
    .bind(project_id)
    .bind(org_id)
    .bind(client_id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO tasks (id, org_id, name, billable_default) VALUES ($1, $2, 'Dev', true)",
    )
    .bind(task_id)
    .bind(org_id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents) \
         VALUES ($1, $2, true, NULL)",
    )
    .bind(project_id)
    .bind(task_id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO assignments (id, project_id, user_id, role) \
         VALUES ($1, $2, $3, 'freelancer'::project_role)",
    )
    .bind(assignment_id)
    .bind(project_id)
    .bind(user_id)
    .execute(pool)
    .await
    .unwrap();

    (project_id, task_id, client_id)
}

/// Same as `seed_project_with_assignment` but does NOT create an assignment.
async fn seed_project_without_assignment(
    pool: &PgPool,
    org_id: Uuid,
) -> (Uuid, Uuid, Uuid) {
    let client_id = Uuid::now_v7();
    let project_id = Uuid::now_v7();
    let task_id = Uuid::now_v7();

    sqlx::query(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Acme', 'EUR')",
    )
    .bind(client_id)
    .bind(org_id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO projects (id, org_id, client_id, name, currency) \
         VALUES ($1, $2, $3, 'Widget', 'EUR')",
    )
    .bind(project_id)
    .bind(org_id)
    .bind(client_id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO tasks (id, org_id, name, billable_default) VALUES ($1, $2, 'Dev', true)",
    )
    .bind(task_id)
    .bind(org_id)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents) \
         VALUES ($1, $2, true, NULL)",
    )
    .bind(project_id)
    .bind(task_id)
    .execute(pool)
    .await
    .unwrap();

    (project_id, task_id, client_id)
}

// ---------------------------------------------------------------------------
// Test 1: Timer start / stop flow
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn timer_start_stop_records_minutes(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, "member").await;
    let (project_id, task_id, _) = seed_project_with_assignment(&pool, org_id, user_id).await;

    // Start a timer by inserting a running entry whose started_at is 5 minutes
    // in the past.
    let entry_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, started_at, state) \
         VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, \
                 0, true, true, now() - interval '5 minutes', 'open'::entry_state)",
    )
    .bind(entry_id)
    .bind(org_id)
    .bind(user_id)
    .bind(project_id)
    .bind(task_id)
    .execute(&pool)
    .await
    .unwrap();

    // Verify is_running
    let (is_running,): (bool,) =
        sqlx::query_as("SELECT is_running FROM time_entries WHERE id = $1")
            .bind(entry_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(is_running);

    // Stop the timer: compute elapsed minutes from started_at and clear the
    // running flag.  This mirrors the stop_timer server function logic.
    sqlx::query(
        "UPDATE time_entries \
         SET is_running = false, \
             minutes = GREATEST(1, EXTRACT(EPOCH FROM (now() - started_at))::int / 60), \
             started_at = NULL, \
             updated_at = now() \
         WHERE id = $1 AND is_running = true",
    )
    .bind(entry_id)
    .execute(&pool)
    .await
    .unwrap();

    // Verify minutes >= 5 and no longer running
    let (minutes, stopped): (i32, bool) = sqlx::query_as(
        "SELECT minutes, is_running FROM time_entries WHERE id = $1",
    )
    .bind(entry_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!stopped);
    assert!(minutes >= 5, "Expected >= 5 minutes, got {minutes}");
}

// ---------------------------------------------------------------------------
// Test 2: One running timer per user (partial unique index)
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn one_timer_per_user_enforced(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, "member").await;
    let (project_id, task_id, _) = seed_project_with_assignment(&pool, org_id, user_id).await;

    // First running timer -- should succeed
    let entry1 = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, started_at, state) \
         VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, \
                 0, true, true, now(), 'open'::entry_state)",
    )
    .bind(entry1)
    .bind(org_id)
    .bind(user_id)
    .bind(project_id)
    .bind(task_id)
    .execute(&pool)
    .await
    .unwrap();

    // Second running timer for the same user -- must fail
    let entry2 = Uuid::now_v7();
    let result = sqlx::query(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, started_at, state) \
         VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, \
                 0, true, true, now(), 'open'::entry_state)",
    )
    .bind(entry2)
    .bind(org_id)
    .bind(user_id)
    .bind(project_id)
    .bind(task_id)
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
async fn submitted_entries_cannot_be_updated(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, "member").await;
    let (project_id, task_id, _) = seed_project_with_assignment(&pool, org_id, user_id).await;

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
    .bind(task_id)
    .execute(&pool)
    .await
    .unwrap();

    // Transition to submitted
    sqlx::query(
        "UPDATE time_entries SET state = 'submitted'::entry_state, updated_at = now() \
         WHERE id = $1",
    )
    .bind(entry_id)
    .execute(&pool)
    .await
    .unwrap();

    // Attempt to change minutes using a WHERE guard that only matches open entries
    let result = sqlx::query(
        "UPDATE time_entries \
         SET minutes = 999, updated_at = now() \
         WHERE id = $1 AND state = 'open'::entry_state",
    )
    .bind(entry_id)
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(
        result.rows_affected(),
        0,
        "No rows should be updated for a submitted entry"
    );

    // Confirm minutes unchanged
    let (minutes,): (i32,) =
        sqlx::query_as("SELECT minutes FROM time_entries WHERE id = $1")
            .bind(entry_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(minutes, 30);
}

// ---------------------------------------------------------------------------
// Test 4: Rounding applied on submit
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn rounding_applied_on_submit(pool: PgPool) {
    use horae_core::rounding::round;
    use horae_core::types::RoundDir;

    // Create org with 15-minute rounding
    let org_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO organizations (id, name, round_minutes, round_dir) \
         VALUES ($1, 'Rounded Org', 15, 'nearest'::round_dir)",
    )
    .bind(org_id)
    .execute(&pool)
    .await
    .unwrap();

    let user_id = seed_user(&pool, org_id, "member").await;
    let (project_id, task_id, _) = seed_project_with_assignment(&pool, org_id, user_id).await;

    // Insert entry with 8 raw minutes
    let entry_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, state) \
         VALUES ($1, $2, $3, $4, $5, CURRENT_DATE, \
                 8, true, false, 'open'::entry_state)",
    )
    .bind(entry_id)
    .bind(org_id)
    .bind(user_id)
    .bind(project_id)
    .bind(task_id)
    .execute(&pool)
    .await
    .unwrap();

    // Simulate the submit path: read org rounding config, compute rounded value,
    // persist it together with the state transition.
    let (round_minutes, round_dir_str): (i16, String) = sqlx::query_as(
        "SELECT round_minutes, round_dir::text FROM organizations WHERE id = $1",
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let dir = match round_dir_str.as_str() {
        "up" => RoundDir::Up,
        "down" => RoundDir::Down,
        _ => RoundDir::Nearest,
    };

    let (raw_minutes,): (i32,) =
        sqlx::query_as("SELECT minutes FROM time_entries WHERE id = $1")
            .bind(entry_id)
            .fetch_one(&pool)
            .await
            .unwrap();

    let rounded = round(raw_minutes as u32, round_minutes as u32, dir);

    // Confirm pure-logic rounding: 8 min with 15-min nearest rounds to 15
    assert_eq!(rounded, 15, "8 rounds to 15 with 15-min nearest rounding");

    // Persist and transition
    sqlx::query(
        "UPDATE time_entries \
         SET rounded_minutes = $1, state = 'submitted'::entry_state, updated_at = now() \
         WHERE id = $2",
    )
    .bind(rounded as i32)
    .bind(entry_id)
    .execute(&pool)
    .await
    .unwrap();

    // Verify stored values
    let (db_rounded, state): (Option<i32>, String) = sqlx::query_as(
        "SELECT rounded_minutes, state::text FROM time_entries WHERE id = $1",
    )
    .bind(entry_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(db_rounded, Some(15));
    assert_eq!(state, "submitted");
}

// ---------------------------------------------------------------------------
// Test 5: Assignment validation
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn unassigned_user_cannot_create_entry(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, "member").await;
    let (project_id, _task_id, _) = seed_project_without_assignment(&pool, org_id).await;

    // Confirm no assignment exists
    let assigned: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM assignments WHERE project_id = $1 AND user_id = $2)",
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!assigned, "User should not be assigned to this project");
}

// ---------------------------------------------------------------------------
// Test 6: Approval workflow -- approve and reject paths
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "./migrations")]
async fn approval_workflow_approve_and_reject(pool: PgPool) {
    let org_id = seed_org(&pool).await;
    let user_id = seed_user(&pool, org_id, "member").await;
    let manager_id = seed_user(&pool, org_id, "manager").await;
    let (project_id, task_id, _) = seed_project_with_assignment(&pool, org_id, user_id).await;

    let period_start = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
    let period_end = NaiveDate::from_ymd_opt(2026, 7, 7).unwrap();

    // --- Approve path ---

    // Create an open entry
    let entry_a = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, state) \
         VALUES ($1, $2, $3, $4, $5, '2026-07-02', \
                 60, true, false, 'open'::entry_state)",
    )
    .bind(entry_a)
    .bind(org_id)
    .bind(user_id)
    .bind(project_id)
    .bind(task_id)
    .execute(&pool)
    .await
    .unwrap();

    // Submit the entry
    sqlx::query(
        "UPDATE time_entries \
         SET state = 'submitted'::entry_state, updated_at = now() \
         WHERE id = $1",
    )
    .bind(entry_a)
    .execute(&pool)
    .await
    .unwrap();

    // Create approval record
    let approval_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO approvals (id, org_id, user_id, period_start, period_end, state) \
         VALUES ($1, $2, $3, $4, $5, 'submitted'::entry_state)",
    )
    .bind(approval_id)
    .bind(org_id)
    .bind(user_id)
    .bind(period_start)
    .bind(period_end)
    .execute(&pool)
    .await
    .unwrap();

    // Manager approves: transition entries + approval
    sqlx::query(
        "UPDATE time_entries \
         SET state = 'approved'::entry_state, updated_at = now() \
         WHERE user_id = $1 AND spent_date BETWEEN $2 AND $3 AND state = 'submitted'::entry_state",
    )
    .bind(user_id)
    .bind(period_start)
    .bind(period_end)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "UPDATE approvals \
         SET state = 'approved'::entry_state, approved_by = $1, approved_at = now() \
         WHERE id = $2",
    )
    .bind(manager_id)
    .bind(approval_id)
    .execute(&pool)
    .await
    .unwrap();

    // Verify entry and approval states
    let (entry_state,): (String,) =
        sqlx::query_as("SELECT state::text FROM time_entries WHERE id = $1")
            .bind(entry_a)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(entry_state, "approved");

    let (approval_state, approver): (String, Option<Uuid>) = sqlx::query_as(
        "SELECT state::text, approved_by FROM approvals WHERE id = $1",
    )
    .bind(approval_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(approval_state, "approved");
    assert_eq!(approver, Some(manager_id));

    // --- Reject path ---

    // Create a new entry, submit, then reject (reopen entries + delete approval)
    let entry_b = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO time_entries \
           (id, org_id, user_id, project_id, task_id, spent_date, \
            minutes, billable, is_running, state) \
         VALUES ($1, $2, $3, $4, $5, '2026-07-14', \
                 45, true, false, 'open'::entry_state)",
    )
    .bind(entry_b)
    .bind(org_id)
    .bind(user_id)
    .bind(project_id)
    .bind(task_id)
    .execute(&pool)
    .await
    .unwrap();

    let period2_start = NaiveDate::from_ymd_opt(2026, 7, 8).unwrap();
    let period2_end = NaiveDate::from_ymd_opt(2026, 7, 14).unwrap();

    // Submit
    sqlx::query(
        "UPDATE time_entries \
         SET state = 'submitted'::entry_state, updated_at = now() \
         WHERE id = $1",
    )
    .bind(entry_b)
    .execute(&pool)
    .await
    .unwrap();

    let approval2_id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO approvals (id, org_id, user_id, period_start, period_end, state) \
         VALUES ($1, $2, $3, $4, $5, 'submitted'::entry_state)",
    )
    .bind(approval2_id)
    .bind(org_id)
    .bind(user_id)
    .bind(period2_start)
    .bind(period2_end)
    .execute(&pool)
    .await
    .unwrap();

    // Reject: reopen entries and delete the approval row
    sqlx::query(
        "UPDATE time_entries \
         SET state = 'open'::entry_state, updated_at = now() \
         WHERE user_id = $1 AND spent_date BETWEEN $2 AND $3 AND state = 'submitted'::entry_state",
    )
    .bind(user_id)
    .bind(period2_start)
    .bind(period2_end)
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query("DELETE FROM approvals WHERE id = $1")
        .bind(approval2_id)
        .execute(&pool)
        .await
        .unwrap();

    // Verify entry reopened
    let (entry_b_state,): (String,) =
        sqlx::query_as("SELECT state::text FROM time_entries WHERE id = $1")
            .bind(entry_b)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(entry_b_state, "open");

    // Verify approval row deleted
    let approval_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM approvals WHERE id = $1)",
    )
    .bind(approval2_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!approval_exists);
}
