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
    #[serde(rename = "time_entry_updated")]
    TimeEntryUpdated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        time_entry: TimeEntryPayload,
    },
    #[serde(rename = "time_entry_deleted")]
    TimeEntryDeleted {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        time_entry: TimeEntryPayload,
    },
    #[serde(rename = "timesheet_submitted")]
    TimesheetSubmitted {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        submission: SubmissionPayload,
    },
    #[serde(rename = "submission_approved")]
    SubmissionApproved {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        submission: SubmissionPayload,
    },
    #[serde(rename = "submission_rejected")]
    SubmissionRejected {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        submission: SubmissionPayload,
    },
    #[serde(rename = "invoice_paid")]
    InvoicePaid {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        invoice: InvoicePayload,
    },
    #[serde(rename = "invoice_voided")]
    InvoiceVoided {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        invoice: InvoicePayload,
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
            Self::TimeEntryUpdated { .. } => "time_entry_updated",
            Self::TimeEntryDeleted { .. } => "time_entry_deleted",
            Self::TimesheetSubmitted { .. } => "timesheet_submitted",
            Self::SubmissionApproved { .. } => "submission_approved",
            Self::SubmissionRejected { .. } => "submission_rejected",
            Self::InvoicePaid { .. } => "invoice_paid",
            Self::InvoiceVoided { .. } => "invoice_voided",
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

#[derive(Debug, Clone, Serialize)]
pub struct SubmissionPayload {
    pub id: Uuid,
    pub user_id: Uuid,
    pub week_start: NaiveDate,
    pub status: String,
    pub total_minutes: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> TimeEntryPayload {
        TimeEntryPayload {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            project_id: Uuid::nil(),
            task_id: Uuid::nil(),
            spent_date: NaiveDate::from_ymd_opt(2026, 7, 13).unwrap(),
            minutes: 30,
            billable: true,
            is_running: false,
            notes: None,
            started_at: None,
        }
    }

    #[test]
    fn new_events_have_stable_hook_names() {
        let org = Uuid::nil();
        let at = DateTime::from_timestamp(0, 0).unwrap();
        let cases = [
            (
                AppEvent::TimeEntryUpdated {
                    occurred_at: at,
                    org_id: org,
                    time_entry: sample_entry(),
                },
                "time_entry_updated",
            ),
            (
                AppEvent::TimeEntryDeleted {
                    occurred_at: at,
                    org_id: org,
                    time_entry: sample_entry(),
                },
                "time_entry_deleted",
            ),
        ];
        for (event, hook) in cases {
            assert_eq!(event.hook_name(), hook);
            // The serde tag in the JSON envelope must match the hook name.
            assert!(
                event.to_json().contains(&format!("\"event\":\"{hook}\"")),
                "envelope tag mismatch for {hook}"
            );
        }
    }

    #[test]
    fn submission_event_serializes_payload() {
        let event = AppEvent::TimesheetSubmitted {
            occurred_at: DateTime::from_timestamp(0, 0).unwrap(),
            org_id: Uuid::nil(),
            submission: SubmissionPayload {
                id: Uuid::nil(),
                user_id: Uuid::nil(),
                week_start: NaiveDate::from_ymd_opt(2026, 7, 6).unwrap(),
                status: "submitted".into(),
                total_minutes: 2280,
            },
        };
        let json = event.to_json();
        assert_eq!(event.hook_name(), "timesheet_submitted");
        assert!(json.contains("\"event\":\"timesheet_submitted\""));
        assert!(json.contains("\"total_minutes\":2280"));
        assert!(json.contains("\"week_start\":\"2026-07-06\""));
    }
}
