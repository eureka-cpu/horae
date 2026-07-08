use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InvoiceStatus {
    Draft,
    Sent,
    Paid,
    Void,
}

impl std::fmt::Display for InvoiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvoiceStatus::Draft => write!(f, "draft"),
            InvoiceStatus::Sent => write!(f, "sent"),
            InvoiceStatus::Paid => write!(f, "paid"),
            InvoiceStatus::Void => write!(f, "void"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Invoice {
    pub id: Uuid,
    pub client_id: Uuid,
    pub invoice_number: String,
    pub status: String,
    pub issued_date: NaiveDate,
    pub due_date: NaiveDate,
    pub total_amount: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
