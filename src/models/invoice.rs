use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Placeholder invoice model — invoicing is Phase 4.
/// Kept here so the invoices page compiles; the list_invoices server fn
/// returns an empty vec until the invoices table is added.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invoice {
    pub id: Uuid,
    pub client_id: Uuid,
    pub invoice_number: String,
    pub status: String,
    pub issued_date: NaiveDate,
    pub due_date: NaiveDate,
    pub total_amount_cents: i64,
}
