//! Project, task, and assignment server functions.

use super::*;

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
    let client_id = parse_uuid(&client_id, "client_id")?;
    let pt = project_type
        .parse::<ProjectType>()
        .map_err(|_| server_err("Invalid project_type"))?;
    let bk = budget_kind
        .parse::<BudgetKind>()
        .map_err(|_| server_err("Invalid budget_kind"))?;
    let project = sqlx::query_as!(
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
    .map_err(server_err)?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::ProjectCreated {
            occurred_at: chrono::Utc::now(),
            org_id: manager.org_id,
            project: project_payload(&project),
        });
    Ok(project)
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
    let project_id = parse_uuid(&project_id, "project_id")?;
    // Detect a real change so a no-op update emits nothing (FR-012).
    let changed: Option<bool> = sqlx::query_scalar::<_, bool>(
        "SELECT (name IS DISTINCT FROM $3
                 OR project_type::text IS DISTINCT FROM $4
                 OR currency IS DISTINCT FROM $5
                 OR budget_kind::text IS DISTINCT FROM $6)
         FROM projects WHERE id = $1 AND org_id = $2",
    )
    .bind(project_id)
    .bind(manager.org_id)
    .bind(&name)
    .bind(&project_type)
    .bind(&currency)
    .bind(&budget_kind)
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?;

    let project = sqlx::query_as::<_, Project>(
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
    .ok_or_else(|| not_found("Project not found"))?;

    if changed == Some(true) {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::ProjectUpdated {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                project: project_payload(&project),
            });
    }
    Ok(project)
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
    let project_id = parse_uuid(&project_id, "project_id")?;
    // Detect a real flip so a no-op set emits nothing (FR-012).
    let was_active: Option<bool> =
        sqlx::query_scalar::<_, bool>("SELECT active FROM projects WHERE id = $1 AND org_id = $2")
            .bind(project_id)
            .bind(manager.org_id)
            .fetch_optional(&state.db)
            .await
            .map_err(server_err)?;

    let project = sqlx::query_as::<_, Project>(
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
    .ok_or_else(|| not_found("Project not found"))?;

    if let Some(t) = crate::plugin::event::active_transition(was_active, active) {
        let occurred_at = chrono::Utc::now();
        let project = project_payload(&project);
        state.plugins.dispatch(match t {
            crate::plugin::event::ActiveTransition::Reactivated => {
                crate::plugin::AppEvent::ProjectReactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    project,
                }
            }
            crate::plugin::event::ActiveTransition::Deactivated => {
                crate::plugin::AppEvent::ProjectDeactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    project,
                }
            }
        });
    }
    Ok(project)
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
    let project_id = parse_uuid(&project_id, "project_id")?;

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
    let task = sqlx::query_as!(
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
    .map_err(server_err)?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::TaskCreated {
            occurred_at: chrono::Utc::now(),
            org_id: manager.org_id,
            task: task_payload(&task),
        });
    Ok(task)
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
    let task_id = parse_uuid(&task_id, "task_id")?;
    // Detect a real change so a no-op update emits nothing (FR-012).
    let changed: Option<bool> = sqlx::query_scalar::<_, bool>(
        "SELECT (name IS DISTINCT FROM $3
                 OR billable_default IS DISTINCT FROM $4
                 OR default_rate_cents IS DISTINCT FROM $5)
         FROM tasks WHERE id = $1 AND org_id = $2",
    )
    .bind(task_id)
    .bind(manager.org_id)
    .bind(&name)
    .bind(billable_default)
    .bind(default_rate_cents)
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?;

    let task = sqlx::query_as::<_, Task>(
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
    .ok_or_else(|| not_found("Task not found"))?;

    if changed == Some(true) {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::TaskUpdated {
                occurred_at: chrono::Utc::now(),
                org_id: manager.org_id,
                task: task_payload(&task),
            });
    }
    Ok(task)
}

/// Activate or deactivate an org-level task. Deactivated tasks are hidden from
/// new-entry pickers but stay attached to existing time entries (FR-011).
#[server]
pub async fn set_task_active(task_id: String, active: bool) -> Result<Task, ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let task_id = parse_uuid(&task_id, "task_id")?;
    // Detect a real flip so a no-op set emits nothing (FR-012).
    let was_active: Option<bool> =
        sqlx::query_scalar::<_, bool>("SELECT active FROM tasks WHERE id = $1 AND org_id = $2")
            .bind(task_id)
            .bind(manager.org_id)
            .fetch_optional(&state.db)
            .await
            .map_err(server_err)?;

    let task = sqlx::query_as::<_, Task>(
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
    .ok_or_else(|| not_found("Task not found"))?;

    if let Some(t) = crate::plugin::event::active_transition(was_active, active) {
        let occurred_at = chrono::Utc::now();
        let task = task_payload(&task);
        state.plugins.dispatch(match t {
            crate::plugin::event::ActiveTransition::Reactivated => {
                crate::plugin::AppEvent::TaskReactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    task,
                }
            }
            crate::plugin::event::ActiveTransition::Deactivated => {
                crate::plugin::AppEvent::TaskDeactivated {
                    occurred_at,
                    org_id: manager.org_id,
                    task,
                }
            }
        });
    }
    Ok(task)
}

/// Enable an org-level task on a project so it becomes loggable there. The
/// project-task link inherits the task's default billable flag; idempotent.
/// Both the project and the task must belong to the manager's organization.
#[server]
pub async fn link_project_task(project_id: String, task_id: String) -> Result<(), ServerFnError> {
    let manager = require_manager().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
    let task_id = parse_uuid(&task_id, "task_id")?;

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
            return Err(not_found("Project or task not found in this organization"));
        }
    }
    Ok(())
}

// ── Assignments ─────────────────────────────────────────────────────────────

#[server]
pub async fn list_assignments(project_id: String) -> Result<Vec<Assignment>, ServerFnError> {
    let _user_id = session_user_id().await?;
    let state = crate::state::global_state().await;
    let project_id = parse_uuid(&project_id, "project_id")?;
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
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let id = uuid::Uuid::now_v7();
    let project_id = parse_uuid(&project_id, "project_id")?;
    let user_id = parse_uuid(&user_id, "user_id")?;
    let pr = role
        .parse::<ProjectRole>()
        .map_err(|_| server_err("Invalid role"))?;
    let assignment = sqlx::query_as!(
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
    .map_err(server_err)?;

    state
        .plugins
        .dispatch(crate::plugin::AppEvent::UserAssignedToProject {
            occurred_at: chrono::Utc::now(),
            org_id: admin.org_id,
            assignment: assignment_payload(&assignment),
        });
    Ok(assignment)
}

#[server]
pub async fn delete_assignment(assignment_id: String) -> Result<(), ServerFnError> {
    let admin = require_admin().await?;
    let state = crate::state::global_state().await;
    let id = parse_uuid(&assignment_id, "assignment_id")?;
    // Delete and capture the row atomically so the event carries its details
    // and a concurrent delete cannot double-notify.
    let removed = sqlx::query_as::<_, crate::models::Assignment>(
        "DELETE FROM assignments WHERE id = $1
         RETURNING id, project_id, user_id, role, rate_cents, created_at",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(server_err)?;

    if let Some(a) = removed {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::AssignmentRemoved {
                occurred_at: chrono::Utc::now(),
                org_id: admin.org_id,
                assignment: assignment_payload(&a),
            });
    }
    Ok(())
}
