use chrono::{DateTime, NaiveDate, Utc};
use horae_core::types::EntryState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct TimeEntry {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub spent_date: NaiveDate,
    /// Precise tracked minutes.
    pub minutes: i32,
    /// Persisted at submit time; None until the entry is locked.
    pub rounded_minutes: Option<i32>,
    pub notes: Option<String>,
    pub billable: bool,
    pub is_running: bool,
    /// Non-null only while is_running = true.
    pub started_at: Option<DateTime<Utc>>,
    pub state: EntryState,
    pub invoice_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TimeEntry {
    /// Format minutes as "H:MM".
    pub fn format_duration(&self) -> String {
        horae_core::duration::format_hhmm(self.minutes as u32)
    }
}
