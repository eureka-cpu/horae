// Harvest-compatible REST API surface, mounted at /harvest/v2.
//
// Tools like harvest-invoicer and harvest-exporter can be pointed at
//   https://horae.example.com/harvest
// and will call /harvest/v2/time_entries etc. as normal.

mod auth;
mod types;

use axum::{Json, Router, extract::Path, extract::Query, routing::get};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use uuid::Uuid;

use auth::AuthUser;
use types::*;

pub fn router() -> Router {
    Router::new().nest(
        "/harvest/v2",
        Router::new()
            .route("/users/me", get(users_me))
            .route("/time_entries", get(list_time_entries))
            .route("/time_entries/{id}", get(get_time_entry))
            .route("/projects", get(list_projects))
            .route("/projects/{id}", get(get_project))
            .route("/clients", get(list_clients))
            .route("/clients/{id}", get(get_client))
            .route("/tasks", get(list_tasks))
            .route("/tasks/{id}", get(get_task))
            .route("/users", get(list_users)),
    )
}

// ── Error helper ────────────────────────────────────────────────────────────

type ApiResult<T> = Result<Json<T>, (axum::http::StatusCode, String)>;

fn internal(e: impl std::fmt::Display) -> (axum::http::StatusCode, String) {
    tracing::error!("Harvest API error: {e}");
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        format!("Internal error: {e}"),
    )
}

fn not_found() -> (axum::http::StatusCode, String) {
    (axum::http::StatusCode::NOT_FOUND, "Not found".to_string())
}

// ── /users/me ───────────────────────────────────────────────────────────────

async fn users_me(user: AuthUser) -> ApiResult<HarvestUser> {
    let state = crate::state::global_state().await;

    let row: UserRow = sqlx::query_as!(
        UserRow,
        r#"SELECT id, name, email, active, org_role::text AS "org_role!: String",
         cost_rate_cents, billable_rate_cents,
         created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM users WHERE id = $1"#,
        user.user_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;

    Ok(Json(user_row_to_harvest(&row)))
}

// ── Time Entries ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TimeEntryFilters {
    pub user_id: Option<String>,
    pub project_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub is_running: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub updated_since: Option<String>,
}

#[derive(sqlx::FromRow)]
struct TimeEntryRow {
    id: Uuid,
    spent_date: NaiveDate,
    minutes: i32,
    rounded_minutes: Option<i32>,
    notes: Option<String>,
    billable: bool,
    is_running: bool,
    started_at: Option<DateTime<Utc>>,
    state: String,
    invoice_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    // Joined fields
    user_id: Uuid,
    user_name: String,
    project_id: Uuid,
    project_name: String,
    project_code: Option<String>,
    task_id: Uuid,
    task_name: String,
    client_id: Uuid,
    client_name: String,
    // Rates
    user_billable_rate_cents: Option<i64>,
    user_cost_rate_cents: Option<i64>,
    budget_kind: String,
}

fn time_entry_row_to_harvest(
    row: &TimeEntryRow,
    org_round_min: u32,
    org_round_dir: horae_core::types::RoundDir,
) -> HarvestTimeEntry {
    let hours = row.minutes as f64 / 60.0;
    let rounded_hours = if let Some(rm) = row.rounded_minutes {
        rm as f64 / 60.0
    } else {
        // Compute rounding for unlocked entries using org settings
        let rounded = horae_core::rounding::round(row.minutes as u32, org_round_min, org_round_dir);
        rounded as f64 / 60.0
    };
    let is_locked = matches!(row.state.as_str(), "submitted" | "approved" | "invoiced");
    let locked_reason = match row.state.as_str() {
        "submitted" => Some("Pending Approval".to_string()),
        "approved" => Some("Approved".to_string()),
        "invoiced" => Some("Invoiced".to_string()),
        _ => None,
    };
    let approval_status = match row.state.as_str() {
        "open" => "unsubmitted",
        "submitted" => "pending_approval",
        "approved" | "invoiced" => "approved",
        other => other,
    };

    HarvestTimeEntry {
        id: row.id.to_string(),
        spent_date: row.spent_date.to_string(),
        hours,
        rounded_hours,
        notes: row.notes.clone(),
        is_locked,
        locked_reason,
        is_closed: is_locked,
        is_billed: row.invoice_id.is_some(),
        is_running: row.is_running,
        timer_started_at: row.started_at.map(|t| t.to_rfc3339()),
        billable: row.billable,
        budgeted: row.budget_kind != "none",
        billable_rate: row.user_billable_rate_cents.map(|c| c as f64 / 100.0),
        cost_rate: row.user_cost_rate_cents.map(|c| c as f64 / 100.0),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.updated_at.to_rfc3339(),
        user: HarvestRef {
            id: row.user_id.to_string(),
            name: row.user_name.clone(),
        },
        client: HarvestRef {
            id: row.client_id.to_string(),
            name: row.client_name.clone(),
        },
        project: HarvestProjectRef {
            id: row.project_id.to_string(),
            name: row.project_name.clone(),
            code: row.project_code.clone(),
        },
        task: HarvestRef {
            id: row.task_id.to_string(),
            name: row.task_name.clone(),
        },
        approval_status: approval_status.to_string(),
    }
}

async fn list_time_entries(
    user: AuthUser,
    Query(filters): Query<TimeEntryFilters>,
) -> ApiResult<HarvestPagination<HarvestTimeEntry>> {
    let state = crate::state::global_state().await;

    // Fetch org rounding config
    let org_row = sqlx::query!(
        r#"SELECT round_minutes, round_dir as "round_dir: horae_core::types::RoundDir"
           FROM organizations WHERE id = $1"#,
        user.org_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;

    let page = filters.page.unwrap_or(1).max(1);
    let per_page = filters.per_page.unwrap_or(100).clamp(1, 100);
    let offset = (page - 1) * per_page;

    // Parse filter strings to properly typed values
    let user_id_filter: Option<Uuid> = filters
        .user_id
        .as_ref()
        .map(|s| s.parse().map_err(|_| internal("Invalid user_id filter")))
        .transpose()?;
    let project_id_filter: Option<Uuid> = filters
        .project_id
        .as_ref()
        .map(|s| s.parse().map_err(|_| internal("Invalid project_id filter")))
        .transpose()?;
    let total_entries = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM time_entries te
         JOIN projects p ON p.id = te.project_id
         WHERE te.org_id = $1
           AND ($2::uuid IS NULL OR te.user_id = $2)
           AND ($3::uuid IS NULL OR te.project_id = $3)
           AND ($4::date IS NULL OR te.spent_date >= $4::date)
           AND ($5::date IS NULL OR te.spent_date <= $5::date)
           AND ($6::bool IS NULL OR te.is_running = $6)
           AND ($7::timestamptz IS NULL OR te.updated_at >= $7::timestamptz)",
        user.org_id,
        user_id_filter,
        project_id_filter,
        filters.from.as_deref() as Option<&str>,
        filters.to.as_deref() as Option<&str>,
        filters.is_running,
        filters.updated_since.as_deref() as Option<&str>,
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?
    .unwrap_or(0);

    let rows = sqlx::query_as!(
        TimeEntryRow,
        r#"SELECT te.id,
               te.spent_date as "spent_date: chrono::NaiveDate",
               te.minutes, te.rounded_minutes, te.notes,
               te.billable, te.is_running,
               te.started_at as "started_at: chrono::DateTime<chrono::Utc>",
               te.state::text AS "state!: String", te.invoice_id,
               te.created_at as "created_at: chrono::DateTime<chrono::Utc>",
               te.updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
               te.user_id, u.name AS user_name,
               te.project_id, p.name AS project_name, p.code AS project_code,
               te.task_id, t.name AS task_name,
               p.client_id, c.name AS client_name,
               u.billable_rate_cents AS user_billable_rate_cents,
               u.cost_rate_cents AS user_cost_rate_cents,
               p.budget_kind::text AS "budget_kind!: String"
         FROM time_entries te
         JOIN users u ON u.id = te.user_id
         JOIN projects p ON p.id = te.project_id
         JOIN tasks t ON t.id = te.task_id
         JOIN clients c ON c.id = p.client_id
         WHERE te.org_id = $1
           AND ($2::uuid IS NULL OR te.user_id = $2)
           AND ($3::uuid IS NULL OR te.project_id = $3)
           AND ($4::date IS NULL OR te.spent_date >= $4::date)
           AND ($5::date IS NULL OR te.spent_date <= $5::date)
           AND ($6::bool IS NULL OR te.is_running = $6)
           AND ($7::timestamptz IS NULL OR te.updated_at >= $7::timestamptz)
         ORDER BY te.spent_date DESC, te.created_at DESC
         LIMIT $8 OFFSET $9"#,
        user.org_id,
        user_id_filter,
        project_id_filter,
        filters.from.as_deref() as Option<&str>,
        filters.to.as_deref() as Option<&str>,
        filters.is_running,
        filters.updated_since.as_deref() as Option<&str>,
        per_page,
        offset,
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;

    let entries: Vec<HarvestTimeEntry> = rows
        .iter()
        .map(|r| time_entry_row_to_harvest(r, org_row.round_minutes as u32, org_row.round_dir))
        .collect();

    Ok(Json(HarvestPagination::new(
        "time_entries",
        entries,
        page,
        per_page,
        total_entries,
        "/harvest/v2/time_entries",
    )))
}

async fn get_time_entry(user: AuthUser, Path(id): Path<Uuid>) -> ApiResult<HarvestTimeEntry> {
    let state = crate::state::global_state().await;

    // Fetch org rounding config
    let org_row = sqlx::query!(
        r#"SELECT round_minutes, round_dir as "round_dir: horae_core::types::RoundDir"
           FROM organizations WHERE id = $1"#,
        user.org_id,
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;

    let row = sqlx::query_as!(
        TimeEntryRow,
        r#"SELECT te.id,
               te.spent_date as "spent_date: chrono::NaiveDate",
               te.minutes, te.rounded_minutes, te.notes,
               te.billable, te.is_running,
               te.started_at as "started_at: chrono::DateTime<chrono::Utc>",
               te.state::text AS "state!: String", te.invoice_id,
               te.created_at as "created_at: chrono::DateTime<chrono::Utc>",
               te.updated_at as "updated_at: chrono::DateTime<chrono::Utc>",
               te.user_id, u.name AS user_name,
               te.project_id, p.name AS project_name, p.code AS project_code,
               te.task_id, t.name AS task_name,
               p.client_id, c.name AS client_name,
               u.billable_rate_cents AS user_billable_rate_cents,
               u.cost_rate_cents AS user_cost_rate_cents,
               p.budget_kind::text AS "budget_kind!: String"
         FROM time_entries te
         JOIN users u ON u.id = te.user_id
         JOIN projects p ON p.id = te.project_id
         JOIN tasks t ON t.id = te.task_id
         JOIN clients c ON c.id = p.client_id
         WHERE te.id = $1 AND te.org_id = $2"#,
        id,
        user.org_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(not_found)?;

    Ok(Json(time_entry_row_to_harvest(
        &row,
        org_row.round_minutes as u32,
        org_row.round_dir,
    )))
}

// ── Projects ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ProjectFilters {
    pub is_active: Option<bool>,
    pub client_id: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub updated_since: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: Uuid,
    name: String,
    code: Option<String>,
    project_type: String,
    active: bool,
    budget_kind: String,
    budget_amount_cents: Option<i64>,
    budget_minutes: Option<i64>,
    starts_on: Option<NaiveDate>,
    ends_on: Option<NaiveDate>,
    created_at: DateTime<Utc>,
    client_id: Uuid,
    client_name: String,
}

fn project_row_to_harvest(row: &ProjectRow) -> HarvestProject {
    let is_billable = row.project_type != "non_billable";
    let bill_by = match row.project_type.as_str() {
        "time_and_materials" => "Tasks",
        "fixed_fee" => "Project",
        "retainer" => "Project",
        _ => "none",
    };
    let budget_by = match row.budget_kind.as_str() {
        "hours" => "person",
        "amount" => "project_cost",
        _ => "none",
    };
    let budget = match row.budget_kind.as_str() {
        "hours" => row.budget_minutes.map(|m| m as f64 / 60.0),
        "amount" => row.budget_amount_cents.map(|c| c as f64 / 100.0),
        _ => None,
    };

    HarvestProject {
        id: row.id.to_string(),
        name: row.name.clone(),
        code: row.code.clone(),
        is_active: row.active,
        is_billable,
        bill_by: bill_by.to_string(),
        budget_by: budget_by.to_string(),
        budget,
        starts_on: row.starts_on.map(|d| d.to_string()),
        ends_on: row.ends_on.map(|d| d.to_string()),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.created_at.to_rfc3339(), // projects table has no updated_at
        client: HarvestRef {
            id: row.client_id.to_string(),
            name: row.client_name.clone(),
        },
    }
}

async fn list_projects(
    user: AuthUser,
    Query(filters): Query<ProjectFilters>,
) -> ApiResult<HarvestPagination<HarvestProject>> {
    let state = crate::state::global_state().await;
    let page = filters.page.unwrap_or(1).max(1);
    let per_page = filters.per_page.unwrap_or(100).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let client_id_filter: Option<Uuid> = filters
        .client_id
        .as_ref()
        .map(|s| s.parse().map_err(|_| internal("Invalid client_id")))
        .transpose()?;

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM projects p
         WHERE p.org_id = $1
           AND ($2::bool IS NULL OR p.active = $2)
           AND ($3::uuid IS NULL OR p.client_id = $3)
           AND ($4::timestamptz IS NULL OR p.created_at >= $4::timestamptz)",
        user.org_id,
        filters.is_active,
        client_id_filter,
        filters.updated_since.as_deref() as Option<&str>,
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?
    .unwrap_or(0);

    let rows = sqlx::query_as!(
        ProjectRow,
        r#"SELECT p.id, p.name, p.code, p.project_type::text AS "project_type!: String", p.active,
         p.budget_kind::text AS "budget_kind!: String", p.budget_amount_cents, p.budget_minutes,
         p.starts_on as "starts_on: chrono::NaiveDate",
         p.ends_on as "ends_on: chrono::NaiveDate",
         p.created_at as "created_at: chrono::DateTime<chrono::Utc>",
         p.client_id, c.name AS client_name
         FROM projects p
         JOIN clients c ON c.id = p.client_id
         WHERE p.org_id = $1
           AND ($2::bool IS NULL OR p.active = $2)
           AND ($3::uuid IS NULL OR p.client_id = $3)
           AND ($4::timestamptz IS NULL OR p.created_at >= $4::timestamptz)
         ORDER BY p.name
         LIMIT $5 OFFSET $6"#,
        user.org_id,
        filters.is_active,
        client_id_filter,
        filters.updated_since.as_deref() as Option<&str>,
        per_page,
        offset,
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;

    let items: Vec<HarvestProject> = rows.iter().map(project_row_to_harvest).collect();

    Ok(Json(HarvestPagination::new(
        "projects",
        items,
        page,
        per_page,
        total,
        "/harvest/v2/projects",
    )))
}

async fn get_project(user: AuthUser, Path(id): Path<Uuid>) -> ApiResult<HarvestProject> {
    let state = crate::state::global_state().await;

    let row = sqlx::query_as!(
        ProjectRow,
        r#"SELECT p.id, p.name, p.code, p.project_type::text AS "project_type!: String", p.active,
         p.budget_kind::text AS "budget_kind!: String", p.budget_amount_cents, p.budget_minutes,
         p.starts_on as "starts_on: chrono::NaiveDate",
         p.ends_on as "ends_on: chrono::NaiveDate",
         p.created_at as "created_at: chrono::DateTime<chrono::Utc>",
         p.client_id, c.name AS client_name
         FROM projects p
         JOIN clients c ON c.id = p.client_id
         WHERE p.id = $1 AND p.org_id = $2"#,
        id,
        user.org_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(not_found)?;

    Ok(Json(project_row_to_harvest(&row)))
}

// ── Clients ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ClientFilters {
    pub is_active: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub updated_since: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ClientRow {
    id: Uuid,
    name: String,
    active: bool,
    address: Option<String>,
    currency: String,
    created_at: DateTime<Utc>,
}

fn client_row_to_harvest(row: &ClientRow) -> HarvestClient {
    HarvestClient {
        id: row.id.to_string(),
        name: row.name.clone(),
        is_active: row.active,
        address: row.address.clone(),
        currency: row.currency.clone(),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.created_at.to_rfc3339(), // no updated_at column
    }
}

async fn list_clients(
    user: AuthUser,
    Query(filters): Query<ClientFilters>,
) -> ApiResult<HarvestPagination<HarvestClient>> {
    let state = crate::state::global_state().await;
    let page = filters.page.unwrap_or(1).max(1);
    let per_page = filters.per_page.unwrap_or(100).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM clients
         WHERE org_id = $1
           AND ($2::bool IS NULL OR active = $2)
           AND ($3::timestamptz IS NULL OR created_at >= $3::timestamptz)",
        user.org_id,
        filters.is_active,
        filters.updated_since.as_deref() as Option<&str>,
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?
    .unwrap_or(0);

    let rows = sqlx::query_as!(
        ClientRow,
        r#"SELECT id, name, active, address, currency,
         created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM clients
         WHERE org_id = $1
           AND ($2::bool IS NULL OR active = $2)
           AND ($3::timestamptz IS NULL OR created_at >= $3::timestamptz)
         ORDER BY name
         LIMIT $4 OFFSET $5"#,
        user.org_id,
        filters.is_active,
        filters.updated_since.as_deref() as Option<&str>,
        per_page,
        offset,
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;

    let items: Vec<HarvestClient> = rows.iter().map(client_row_to_harvest).collect();

    Ok(Json(HarvestPagination::new(
        "clients",
        items,
        page,
        per_page,
        total,
        "/harvest/v2/clients",
    )))
}

async fn get_client(user: AuthUser, Path(id): Path<Uuid>) -> ApiResult<HarvestClient> {
    let state = crate::state::global_state().await;

    let row = sqlx::query_as!(
        ClientRow,
        r#"SELECT id, name, active, address, currency,
         created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM clients WHERE id = $1 AND org_id = $2"#,
        id,
        user.org_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(not_found)?;

    Ok(Json(client_row_to_harvest(&row)))
}

// ── Tasks ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TaskFilters {
    pub is_active: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    #[allow(dead_code)]
    pub updated_since: Option<String>,
}

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: Uuid,
    name: String,
    active: bool,
    billable_default: bool,
    default_rate_cents: Option<i64>,
}

fn task_row_to_harvest(row: &TaskRow) -> HarvestTask {
    HarvestTask {
        id: row.id.to_string(),
        name: row.name.clone(),
        is_active: row.active,
        billable_by_default: row.billable_default,
        default_hourly_rate: row.default_rate_cents.map(|c| c as f64 / 100.0),
        created_at: String::new(), // tasks table has no created_at
        updated_at: String::new(),
    }
}

async fn list_tasks(
    user: AuthUser,
    Query(filters): Query<TaskFilters>,
) -> ApiResult<HarvestPagination<HarvestTask>> {
    let state = crate::state::global_state().await;
    let page = filters.page.unwrap_or(1).max(1);
    let per_page = filters.per_page.unwrap_or(100).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM tasks
         WHERE org_id = $1
           AND ($2::bool IS NULL OR active = $2)",
        user.org_id,
        filters.is_active,
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?
    .unwrap_or(0);

    let rows = sqlx::query_as!(
        TaskRow,
        "SELECT id, name, active, billable_default, default_rate_cents
         FROM tasks
         WHERE org_id = $1
           AND ($2::bool IS NULL OR active = $2)
         ORDER BY name
         LIMIT $3 OFFSET $4",
        user.org_id,
        filters.is_active,
        per_page,
        offset,
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;

    let items: Vec<HarvestTask> = rows.iter().map(task_row_to_harvest).collect();

    Ok(Json(HarvestPagination::new(
        "tasks",
        items,
        page,
        per_page,
        total,
        "/harvest/v2/tasks",
    )))
}

async fn get_task(user: AuthUser, Path(id): Path<Uuid>) -> ApiResult<HarvestTask> {
    let state = crate::state::global_state().await;

    let row = sqlx::query_as!(
        TaskRow,
        "SELECT id, name, active, billable_default, default_rate_cents
         FROM tasks WHERE id = $1 AND org_id = $2",
        id,
        user.org_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(internal)?
    .ok_or_else(not_found)?;

    Ok(Json(task_row_to_harvest(&row)))
}

// ── Users ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UserFilters {
    pub is_active: Option<bool>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    #[allow(dead_code)]
    pub updated_since: Option<String>,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    name: String,
    email: String,
    active: bool,
    org_role: String,
    cost_rate_cents: Option<i64>,
    billable_rate_cents: Option<i64>,
    created_at: DateTime<Utc>,
}

fn user_row_to_harvest(row: &UserRow) -> HarvestUser {
    // Split "First Last" into first_name / last_name
    let (first, last) = match row.name.split_once(' ') {
        Some((f, l)) => (f.to_string(), l.to_string()),
        None => (row.name.clone(), String::new()),
    };

    HarvestUser {
        id: row.id.to_string(),
        first_name: first,
        last_name: last,
        email: row.email.clone(),
        is_active: row.active,
        is_admin: row.org_role == "admin",
        cost_rate: row.cost_rate_cents.map(|c| c as f64 / 100.0),
        default_hourly_rate: row.billable_rate_cents.map(|c| c as f64 / 100.0),
        created_at: row.created_at.to_rfc3339(),
        updated_at: row.created_at.to_rfc3339(), // no updated_at column
    }
}

async fn list_users(
    user: AuthUser,
    Query(filters): Query<UserFilters>,
) -> ApiResult<HarvestPagination<HarvestUser>> {
    let state = crate::state::global_state().await;
    let page = filters.page.unwrap_or(1).max(1);
    let per_page = filters.per_page.unwrap_or(100).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM users
         WHERE org_id = $1
           AND ($2::bool IS NULL OR active = $2)",
        user.org_id,
        filters.is_active,
    )
    .fetch_one(&state.db)
    .await
    .map_err(internal)?
    .unwrap_or(0);

    let rows = sqlx::query_as!(
        UserRow,
        r#"SELECT id, name, email, active, org_role::text AS "org_role!: String",
         cost_rate_cents, billable_rate_cents,
         created_at as "created_at: chrono::DateTime<chrono::Utc>"
         FROM users
         WHERE org_id = $1
           AND ($2::bool IS NULL OR active = $2)
         ORDER BY name
         LIMIT $3 OFFSET $4"#,
        user.org_id,
        filters.is_active,
        per_page,
        offset,
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal)?;

    let items: Vec<HarvestUser> = rows.iter().map(user_row_to_harvest).collect();

    Ok(Json(HarvestPagination::new(
        "users",
        items,
        page,
        per_page,
        total,
        "/harvest/v2/users",
    )))
}
