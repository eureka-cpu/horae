use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BillingMethod {
    Hourly,
    Fixed,
    NonBillable,
}

impl std::fmt::Display for BillingMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BillingMethod::Hourly => write!(f, "hourly"),
            BillingMethod::Fixed => write!(f, "fixed"),
            BillingMethod::NonBillable => write!(f, "non_billable"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Project {
    pub id: Uuid,
    pub client_id: Uuid,
    pub name: String,
    pub code: Option<String>,
    pub budget_hours: Option<f64>,
    pub billing_method: String,
    pub hourly_rate: f64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
