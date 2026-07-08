use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub default_currency: String,
    pub week_start: i16,
    pub round_minutes: i16,
    /// "nearest" | "up" | "down"
    pub round_dir: String,
    pub created_at: DateTime<Utc>,
}
