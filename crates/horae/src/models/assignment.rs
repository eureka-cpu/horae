use chrono::{DateTime, Utc};
use horae_core::types::ProjectRole;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Assignment {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub role: ProjectRole,
    pub rate_cents: Option<i64>,
    pub created_at: DateTime<Utc>,
}
