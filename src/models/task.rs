use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An org-level task (global catalog entry). Tasks are enabled per project via
/// `project_tasks`; a task row here does NOT belong to a specific project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Task {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub billable_default: bool,
    pub default_rate_cents: Option<i64>,
    pub active: bool,
}
