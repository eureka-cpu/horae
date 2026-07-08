use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Task {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub hourly_rate: f64,
    pub is_billable: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
