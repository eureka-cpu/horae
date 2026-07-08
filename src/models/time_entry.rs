use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct TimeEntry {
    pub id: Uuid,
    pub user_id: Uuid,
    pub project_id: Uuid,
    pub task_id: Option<Uuid>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_seconds: i64,
    pub notes: Option<String>,
    pub is_billable: bool,
    pub invoice_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TimeEntry {
    /// Returns true if the timer is still running (no ended_at)
    pub fn is_running(&self) -> bool {
        self.ended_at.is_none()
    }

    /// Computes elapsed seconds (for running timers, from started_at to now)
    pub fn elapsed_seconds(&self) -> i64 {
        if let Some(ended) = self.ended_at {
            (ended - self.started_at).num_seconds()
        } else {
            (Utc::now() - self.started_at).num_seconds()
        }
    }
}
