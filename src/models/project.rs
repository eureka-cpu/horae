use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A project row. Enum columns (`project_type`, `budget_kind`) are selected as
/// `col::text` and stored here as `String`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Project {
    pub id: Uuid,
    pub org_id: Uuid,
    pub client_id: Uuid,
    pub code: Option<String>,
    pub name: String,
    /// "time_and_materials" | "fixed_fee" | "non_billable" | "retainer"
    pub project_type: String,
    pub currency: String,
    pub starts_on: Option<NaiveDate>,
    pub ends_on: Option<NaiveDate>,
    /// "none" | "amount" | "hours"
    pub budget_kind: String,
    pub budget_amount_cents: Option<i64>,
    pub budget_minutes: Option<i64>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}
