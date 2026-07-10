use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Client {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub currency: String,
    pub address: Option<String>,
    pub tax_id: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}
