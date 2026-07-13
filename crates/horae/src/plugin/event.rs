use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use uuid::Uuid;

/// Business events dispatched to subscribed plugins (FR-019).
/// Each variant carries a payload matching the contract's JSON schemas.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event")]
pub enum AppEvent {
    #[serde(rename = "time_entry_created")]
    TimeEntryCreated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        time_entry: TimeEntryPayload,
    },
    #[serde(rename = "time_entry_stopped")]
    TimeEntryStopped {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        time_entry: TimeEntryPayload,
    },
    #[serde(rename = "invoice_created")]
    InvoiceCreated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        invoice: InvoicePayload,
    },
    #[serde(rename = "invoice_sent")]
    InvoiceSent {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        invoice: InvoicePayload,
    },
    #[serde(rename = "user_logged_in")]
    UserLoggedIn {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        user: UserPayload,
    },
}

impl AppEvent {
    /// The hook name matching the plugin manifest's `hooks` entries.
    pub fn hook_name(&self) -> &'static str {
        match self {
            Self::TimeEntryCreated { .. } => "time_entry_created",
            Self::TimeEntryStopped { .. } => "time_entry_stopped",
            Self::InvoiceCreated { .. } => "invoice_created",
            Self::InvoiceSent { .. } => "invoice_sent",
            Self::UserLoggedIn { .. } => "user_logged_in",
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeEntryPayload {
    pub id: Uuid,
    pub user_id: Uuid,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub spent_date: NaiveDate,
    pub minutes: i32,
    pub billable: bool,
    pub is_running: bool,
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvoicePayload {
    pub id: Uuid,
    pub client_id: Uuid,
    pub invoice_number: String,
    pub status: String,
    pub issue_date: NaiveDate,
    pub due_date: NaiveDate,
    pub currency: String,
    pub total_cents: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserPayload {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub org_role: String,
    pub method: String,
}
