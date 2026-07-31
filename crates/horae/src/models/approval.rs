use chrono::{DateTime, NaiveDate, Utc};
use horae_core::types::EntryState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Approval {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub state: EntryState,
    pub submitted_at: DateTime<Utc>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
}

/// An approval row plus the tracked time for its period, for the review table.
/// Hours are not stored on the approval — they are aggregated from the user's
/// `time_entries` within `[period_start, period_end]` (actual `minutes`, not the
/// invoice-time `rounded_minutes`), split into billable and total.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalSummary {
    pub approval: Approval,
    pub total_minutes: i64,
    pub billable_minutes: i64,
}
