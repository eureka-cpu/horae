// Harvest-compatible REST API surface, mounted at /harvest/v2.
//
// Tools like harvest-invoicer and harvest-exporter can be pointed at
//   https://horae.example.com/harvest
// and will call /harvest/v2/time_entries etc. as normal.

mod auth;
mod types;

use axum::{extract::Path, extract::Query, routing::get, Json, Router};
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
    (
        axum::http::StatusCode::NOT_FOUND,
        "Not found".to_string(),
    )
}

// ── /users/me ───────────────────────────────────────────────────────────────

async fn users_me(user: AuthUser) -> ApiResult<HarvestUser> {
    let state = crate::state::global_state().await;

    let row: UserRow = sqlx::query_as(
        "SELECT id, name, email, active, org_role::text AS org_role, \
         cost_rate_cents, billable_rate_cents, created_at \
         FROM users WHERE id = $1",
    )
    .bind(user.user_id)
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

const TIME_ENTRY_SELECT: &str = "\
    SELECT te.id, te.spent_date, te.minutes, te.rounded_minutes, te.notes, \
           te.billable, te.is_running, te.started_at, \
           te.state::text AS state, te.invoice_id, \
           te.created_at, te.updated_at, \
           te.user_id, u.name AS user_name, \
           te.project_id, p.name AS project_name, p.code AS project_code, \
           te.task_id, t.name AS task_name, \
           p.client_id, c.name AS client_name, \
           u.billable_rate_cents AS user_billable_rate_cents, \
           u.cost_rate_cents AS user_cost_rate_cents, \
           p.budget_kind::text AS budget_kind \
    FROM time_entries te \
    JOIN users u ON u.id = te.user_id \
    JOIN projects p ON p.id = te.project_id \
    JOIN tasks t ON t.id = te.task_id \
    JOIN clients c ON c.id = p.client_id";

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
        billable_rate: row
            .user_billable_rate_cents
            .map(|c| c as f64 / 100.0),
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
    let (org_round_min, org_round_dir_str): (i16, String) = sqlx::query_as(
        "SELECT round_minutes, round_dir::text FROM organizations WHERE id = $1"
    )
    .bind(user.org_id)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    let org_round_dir = match org_round_dir_str.as_str() {
        "up" => horae_core::types::RoundDir::Up,
        "down" => horae_core::types::RoundDir::Down,
        _ => horae_core::types::RoundDir::Nearest,
    };

    let page = filters.page.unwrap_or(1).max(1);
    let per_page = filters.per_page.unwrap_or(100).clamp(1, 100);
    let offset = (page - 1) * per_page;

    // Build dynamic WHERE clause
    let mut conditions = vec!["te.org_id = $1".to_string()];
    let mut param_idx = 2u32;

    // We collect bind values as strings so we can bind them uniformly.
    // UUIDs and dates are passed as text and cast in the query.
    struct Param {
        value: String,
        _kind: ParamKind,
    }
    enum ParamKind {
        Uuid,
        Date,
        Timestamp,
        Bool,
    }

    let mut extra_params: Vec<Param> = Vec::new();

    if let Some(ref uid) = filters.user_id {
        conditions.push(format!("te.user_id = ${param_idx}::uuid"));
        extra_params.push(Param {
            value: uid.clone(),
            _kind: ParamKind::Uuid,
        });
        param_idx += 1;
    }
    if let Some(ref pid) = filters.project_id {
        conditions.push(format!("te.project_id = ${param_idx}::uuid"));
        extra_params.push(Param {
            value: pid.clone(),
            _kind: ParamKind::Uuid,
        });
        param_idx += 1;
    }
    if let Some(ref from) = filters.from {
        conditions.push(format!("te.spent_date >= ${param_idx}::date"));
        extra_params.push(Param {
            value: from.clone(),
            _kind: ParamKind::Date,
        });
        param_idx += 1;
    }
    if let Some(ref to) = filters.to {
        conditions.push(format!("te.spent_date <= ${param_idx}::date"));
        extra_params.push(Param {
            value: to.clone(),
            _kind: ParamKind::Date,
        });
        param_idx += 1;
    }
    if let Some(running) = filters.is_running {
        conditions.push(format!("te.is_running = ${param_idx}::bool"));
        extra_params.push(Param {
            value: running.to_string(),
            _kind: ParamKind::Bool,
        });
        param_idx += 1;
    }
    if let Some(ref since) = filters.updated_since {
        conditions.push(format!("te.updated_at >= ${param_idx}::timestamptz"));
        extra_params.push(Param {
            value: since.clone(),
            _kind: ParamKind::Timestamp,
        });
        param_idx += 1;
    }

    let where_clause = conditions.join(" AND ");

    // Count query
    let count_sql = format!(
        "SELECT COUNT(*) FROM time_entries te \
         JOIN projects p ON p.id = te.project_id \
         WHERE {where_clause}"
    );

    let data_sql = format!(
        "{TIME_ENTRY_SELECT} WHERE {where_clause} \
         ORDER BY te.spent_date DESC, te.created_at DESC \
         LIMIT ${param_idx} OFFSET ${}",
        param_idx + 1
    );

    // Bind parameters to count query
    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql).bind(user.org_id);
    for p in &extra_params {
        count_query = count_query.bind(&p.value);
    }
    let total_entries: i64 = count_query.fetch_one(&state.db).await.map_err(internal)?;

    // Bind parameters to data query
    let mut data_query = sqlx::query_as::<_, TimeEntryRow>(&data_sql).bind(user.org_id);
    for p in &extra_params {
        data_query = data_query.bind(&p.value);
    }
    data_query = data_query.bind(per_page).bind(offset);

    let rows: Vec<TimeEntryRow> = data_query.fetch_all(&state.db).await.map_err(internal)?;

    let entries: Vec<HarvestTimeEntry> = rows.iter().map(|r| time_entry_row_to_harvest(r, org_round_min as u32, org_round_dir)).collect();

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
    let (org_round_min, org_round_dir_str): (i16, String) = sqlx::query_as(
        "SELECT round_minutes, round_dir::text FROM organizations WHERE id = $1"
    )
    .bind(user.org_id)
    .fetch_one(&state.db)
    .await
    .map_err(internal)?;
    let org_round_dir = match org_round_dir_str.as_str() {
        "up" => horae_core::types::RoundDir::Up,
        "down" => horae_core::types::RoundDir::Down,
        _ => horae_core::types::RoundDir::Nearest,
    };

    let sql = format!("{TIME_ENTRY_SELECT} WHERE te.id = $1 AND te.org_id = $2");
    let row: TimeEntryRow = sqlx::query_as(&sql)
        .bind(id)
        .bind(user.org_id)
        .fetch_optional(&state.db)
        .await
        .map_err(internal)?
        .ok_or_else(not_found)?;

    Ok(Json(time_entry_row_to_harvest(&row, org_round_min as u32, org_round_dir)))
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

    let mut conditions = vec!["p.org_id = $1".to_string()];
    let mut extra_values: Vec<String> = Vec::new();
    let mut param_idx = 2u32;

    if let Some(active) = filters.is_active {
        conditions.push(format!("p.active = ${param_idx}::bool"));
        extra_values.push(active.to_string());
        param_idx += 1;
    }
    if let Some(ref cid) = filters.client_id {
        conditions.push(format!("p.client_id = ${param_idx}::uuid"));
        extra_values.push(cid.clone());
        param_idx += 1;
    }
    if let Some(ref since) = filters.updated_since {
        conditions.push(format!("p.created_at >= ${param_idx}::timestamptz"));
        extra_values.push(since.clone());
        param_idx += 1;
    }

    let where_clause = conditions.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) FROM projects p WHERE {where_clause}");
    let data_sql = format!(
        "SELECT p.id, p.name, p.code, p.project_type::text AS project_type, p.active, \
         p.budget_kind::text AS budget_kind, p.budget_amount_cents, p.budget_minutes, \
         p.starts_on, p.ends_on, p.created_at, \
         p.client_id, c.name AS client_name \
         FROM projects p \
         JOIN clients c ON c.id = p.client_id \
         WHERE {where_clause} \
         ORDER BY p.name \
         LIMIT ${param_idx} OFFSET ${}",
        param_idx + 1
    );

    let mut cq = sqlx::query_scalar::<_, i64>(&count_sql).bind(user.org_id);
    for v in &extra_values {
        cq = cq.bind(v);
    }
    let total: i64 = cq.fetch_one(&state.db).await.map_err(internal)?;

    let mut dq = sqlx::query_as::<_, ProjectRow>(&data_sql).bind(user.org_id);
    for v in &extra_values {
        dq = dq.bind(v);
    }
    dq = dq.bind(per_page).bind(offset);
    let rows: Vec<ProjectRow> = dq.fetch_all(&state.db).await.map_err(internal)?;

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

    let row: ProjectRow = sqlx::query_as(
        "SELECT p.id, p.name, p.code, p.project_type::text AS project_type, p.active, \
         p.budget_kind::text AS budget_kind, p.budget_amount_cents, p.budget_minutes, \
         p.starts_on, p.ends_on, p.created_at, \
         p.client_id, c.name AS client_name \
         FROM projects p \
         JOIN clients c ON c.id = p.client_id \
         WHERE p.id = $1 AND p.org_id = $2",
    )
    .bind(id)
    .bind(user.org_id)
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

    let mut conditions = vec!["org_id = $1".to_string()];
    let mut extra_values: Vec<String> = Vec::new();
    let mut param_idx = 2u32;

    if let Some(active) = filters.is_active {
        conditions.push(format!("active = ${param_idx}::bool"));
        extra_values.push(active.to_string());
        param_idx += 1;
    }
    if let Some(ref since) = filters.updated_since {
        conditions.push(format!("created_at >= ${param_idx}::timestamptz"));
        extra_values.push(since.clone());
        param_idx += 1;
    }

    let where_clause = conditions.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) FROM clients WHERE {where_clause}");
    let data_sql = format!(
        "SELECT id, name, active, address, currency, created_at \
         FROM clients WHERE {where_clause} ORDER BY name \
         LIMIT ${param_idx} OFFSET ${}",
        param_idx + 1
    );

    let mut cq = sqlx::query_scalar::<_, i64>(&count_sql).bind(user.org_id);
    for v in &extra_values {
        cq = cq.bind(v);
    }
    let total: i64 = cq.fetch_one(&state.db).await.map_err(internal)?;

    let mut dq = sqlx::query_as::<_, ClientRow>(&data_sql).bind(user.org_id);
    for v in &extra_values {
        dq = dq.bind(v);
    }
    dq = dq.bind(per_page).bind(offset);
    let rows: Vec<ClientRow> = dq.fetch_all(&state.db).await.map_err(internal)?;

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

    let row: ClientRow = sqlx::query_as(
        "SELECT id, name, active, address, currency, created_at \
         FROM clients WHERE id = $1 AND org_id = $2",
    )
    .bind(id)
    .bind(user.org_id)
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

    let mut conditions = vec!["org_id = $1".to_string()];
    let mut extra_values: Vec<String> = Vec::new();
    let mut param_idx = 2u32;

    if let Some(active) = filters.is_active {
        conditions.push(format!("active = ${param_idx}::bool"));
        extra_values.push(active.to_string());
        param_idx += 1;
    }

    let where_clause = conditions.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) FROM tasks WHERE {where_clause}");
    let data_sql = format!(
        "SELECT id, name, active, billable_default, default_rate_cents \
         FROM tasks WHERE {where_clause} ORDER BY name \
         LIMIT ${param_idx} OFFSET ${}",
        param_idx + 1
    );

    let mut cq = sqlx::query_scalar::<_, i64>(&count_sql).bind(user.org_id);
    for v in &extra_values {
        cq = cq.bind(v);
    }
    let total: i64 = cq.fetch_one(&state.db).await.map_err(internal)?;

    let mut dq = sqlx::query_as::<_, TaskRow>(&data_sql).bind(user.org_id);
    for v in &extra_values {
        dq = dq.bind(v);
    }
    dq = dq.bind(per_page).bind(offset);
    let rows: Vec<TaskRow> = dq.fetch_all(&state.db).await.map_err(internal)?;

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

    let row: TaskRow = sqlx::query_as(
        "SELECT id, name, active, billable_default, default_rate_cents \
         FROM tasks WHERE id = $1 AND org_id = $2",
    )
    .bind(id)
    .bind(user.org_id)
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

    let mut conditions = vec!["org_id = $1".to_string()];
    let mut extra_values: Vec<String> = Vec::new();
    let mut param_idx = 2u32;

    if let Some(active) = filters.is_active {
        conditions.push(format!("active = ${param_idx}::bool"));
        extra_values.push(active.to_string());
        param_idx += 1;
    }

    let where_clause = conditions.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) FROM users WHERE {where_clause}");
    let data_sql = format!(
        "SELECT id, name, email, active, org_role::text AS org_role, \
         cost_rate_cents, billable_rate_cents, created_at \
         FROM users WHERE {where_clause} ORDER BY name \
         LIMIT ${param_idx} OFFSET ${}",
        param_idx + 1
    );

    let mut cq = sqlx::query_scalar::<_, i64>(&count_sql).bind(user.org_id);
    for v in &extra_values {
        cq = cq.bind(v);
    }
    let total: i64 = cq.fetch_one(&state.db).await.map_err(internal)?;

    let mut dq = sqlx::query_as::<_, UserRow>(&data_sql).bind(user.org_id);
    for v in &extra_values {
        dq = dq.bind(v);
    }
    dq = dq.bind(per_page).bind(offset);
    let rows: Vec<UserRow> = dq.fetch_all(&state.db).await.map_err(internal)?;

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
