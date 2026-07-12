use chrono::{DateTime, NaiveDate, Utc};
use horae_core::types::InvoiceStatus;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Invoice {
    pub id: Uuid,
    pub org_id: Uuid,
    pub client_id: Uuid,
    pub number: String,
    pub status: InvoiceStatus,
    pub issued_on: NaiveDate,
    pub due_on: NaiveDate,
    pub currency: String,
    pub total_cents: i64,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct InvoiceLine {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub time_entry_id: Uuid,
    pub description: String,
    pub minutes: i32,
    pub rate_cents: i64,
    pub amount_cents: i64,
}

/// Invoice with its line items, returned by `get_invoice`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceWithLines {
    pub invoice: Invoice,
    pub lines: Vec<InvoiceLine>,
}
