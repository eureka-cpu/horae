use chrono::{DateTime, Utc};
use horae_core::types::OrgRole;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct User {
    pub id: Uuid,
    pub org_id: Uuid,
    pub email: String,
    pub name: String,
    pub oidc_subject: Option<String>,
    pub org_role: OrgRole,
    pub cost_rate_cents: Option<i64>,
    pub billable_rate_cents: Option<i64>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn is_admin(&self) -> bool {
        self.org_role == OrgRole::Admin
    }

    pub fn is_manager_or_above(&self) -> bool {
        matches!(self.org_role, OrgRole::Admin | OrgRole::Manager)
    }
}
