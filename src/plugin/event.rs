use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AppEvent {
    TimeEntryCreated { entry_id: Uuid, user_id: Uuid, project_id: Uuid },
    TimeEntryStopped { entry_id: Uuid, duration_seconds: i64 },
    InvoiceCreated { invoice_id: Uuid, client_id: Uuid },
    InvoiceSent { invoice_id: Uuid },
    UserLoggedIn { user_id: Uuid, email: String },
}

impl AppEvent {
    pub fn hook_name(&self) -> &'static str {
        match self {
            AppEvent::TimeEntryCreated { .. } => "time_entry_created",
            AppEvent::TimeEntryStopped { .. } => "time_entry_stopped",
            AppEvent::InvoiceCreated { .. } => "invoice_created",
            AppEvent::InvoiceSent { .. } => "invoice_sent",
            AppEvent::UserLoggedIn { .. } => "user_logged_in",
        }
    }
}
