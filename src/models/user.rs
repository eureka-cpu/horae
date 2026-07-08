use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A user row from the `users` table.
///
/// `org_role` is stored as a Postgres enum but selected as `org_role::text` so it
/// decodes into a plain `String` without requiring a custom sqlx type registration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct User {
    pub id: Uuid,
    pub org_id: Uuid,
    pub email: String,
    pub name: String,
    pub oidc_subject: Option<String>,
    /// "admin" | "manager" | "member"
    pub org_role: String,
    pub cost_rate_cents: Option<i64>,
    pub billable_rate_cents: Option<i64>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.org_role == "admin"
    }

    pub fn is_manager_or_above(&self) -> bool {
        matches!(self.org_role.as_str(), "admin" | "manager")
    }
}
