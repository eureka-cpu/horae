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
    #[serde(rename = "client_created")]
    ClientCreated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        client: ClientPayload,
    },
    #[serde(rename = "client_updated")]
    ClientUpdated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        client: ClientPayload,
    },
    #[serde(rename = "client_deactivated")]
    ClientDeactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        client: ClientPayload,
    },
    #[serde(rename = "client_reactivated")]
    ClientReactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        client: ClientPayload,
    },
    #[serde(rename = "project_created")]
    ProjectCreated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        project: ProjectPayload,
    },
    #[serde(rename = "project_updated")]
    ProjectUpdated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        project: ProjectPayload,
    },
    #[serde(rename = "project_deactivated")]
    ProjectDeactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        project: ProjectPayload,
    },
    #[serde(rename = "project_reactivated")]
    ProjectReactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        project: ProjectPayload,
    },
    #[serde(rename = "task_created")]
    TaskCreated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        task: TaskPayload,
    },
    #[serde(rename = "task_updated")]
    TaskUpdated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        task: TaskPayload,
    },
    #[serde(rename = "task_deactivated")]
    TaskDeactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        task: TaskPayload,
    },
    #[serde(rename = "task_reactivated")]
    TaskReactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        task: TaskPayload,
    },
    #[serde(rename = "user_created")]
    UserCreated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        user: UserPayload,
    },
    #[serde(rename = "user_role_changed")]
    UserRoleChanged {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        user: UserPayload,
        previous_role: String,
    },
    #[serde(rename = "user_deactivated")]
    UserDeactivated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        user: UserPayload,
    },
    #[serde(rename = "user_logged_out")]
    UserLoggedOut {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        user: UserPayload,
    },
    #[serde(rename = "user_assigned_to_project")]
    UserAssignedToProject {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        assignment: AssignmentPayload,
    },
    #[serde(rename = "assignment_removed")]
    AssignmentRemoved {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        assignment: AssignmentPayload,
    },
    #[serde(rename = "org_branding_updated")]
    OrgBrandingUpdated {
        occurred_at: DateTime<Utc>,
        org_id: Uuid,
        org: OrgBrandingPayload,
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
            Self::ClientCreated { .. } => "client_created",
            Self::ClientUpdated { .. } => "client_updated",
            Self::ClientDeactivated { .. } => "client_deactivated",
            Self::ClientReactivated { .. } => "client_reactivated",
            Self::ProjectCreated { .. } => "project_created",
            Self::ProjectUpdated { .. } => "project_updated",
            Self::ProjectDeactivated { .. } => "project_deactivated",
            Self::ProjectReactivated { .. } => "project_reactivated",
            Self::TaskCreated { .. } => "task_created",
            Self::TaskUpdated { .. } => "task_updated",
            Self::TaskDeactivated { .. } => "task_deactivated",
            Self::TaskReactivated { .. } => "task_reactivated",
            Self::UserCreated { .. } => "user_created",
            Self::UserRoleChanged { .. } => "user_role_changed",
            Self::UserDeactivated { .. } => "user_deactivated",
            Self::UserLoggedOut { .. } => "user_logged_out",
            Self::UserAssignedToProject { .. } => "user_assigned_to_project",
            Self::AssignmentRemoved { .. } => "assignment_removed",
            Self::OrgBrandingUpdated { .. } => "org_branding_updated",
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
    /// How the user authenticated — set only on login events; omitted otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmissionPayload {
    pub id: Uuid,
    pub user_id: Uuid,
    pub week_start: NaiveDate,
    pub status: String,
    pub total_minutes: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientPayload {
    pub id: Uuid,
    pub name: String,
    pub currency: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectPayload {
    pub id: Uuid,
    pub client_id: Uuid,
    pub name: String,
    pub project_type: String,
    pub budget_kind: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskPayload {
    pub id: Uuid,
    pub name: String,
    pub billable_default: bool,
    pub default_rate_cents: Option<i64>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssignmentPayload {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
}

/// Only non-sensitive branding fields are exposed to plugins (no bank details).
#[derive(Debug, Clone, Serialize)]
pub struct OrgBrandingPayload {
    pub org_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_name: Option<String>,
}

/// Which lifecycle event an active-flag write should emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTransition {
    Deactivated,
    Reactivated,
}

/// Decide which event an `active` write represents. Returns `None` when the flag
/// did not actually change, so a no-op set emits nothing (FR-012). `was_active`
/// is the value read before the write (`None` if the row was absent).
pub fn active_transition(was_active: Option<bool>, now_active: bool) -> Option<ActiveTransition> {
    match was_active {
        Some(prev) if prev != now_active => Some(if now_active {
            ActiveTransition::Reactivated
        } else {
            ActiveTransition::Deactivated
        }),
        _ => None,
    }
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

    #[test]
    fn us2_events_have_stable_hook_names() {
        let org = Uuid::nil();
        let at = DateTime::from_timestamp(0, 0).unwrap();
        let client = ClientPayload {
            id: Uuid::nil(),
            name: "Acme".into(),
            currency: "EUR".into(),
            active: false,
        };
        let e = AppEvent::ClientDeactivated {
            occurred_at: at,
            org_id: org,
            client: client.clone(),
        };
        assert_eq!(e.hook_name(), "client_deactivated");
        assert!(e.to_json().contains("\"event\":\"client_deactivated\""));
        assert_eq!(
            AppEvent::ClientReactivated {
                occurred_at: at,
                org_id: org,
                client,
            }
            .hook_name(),
            "client_reactivated"
        );
    }

    #[test]
    fn role_change_event_carries_previous_role() {
        let event = AppEvent::UserRoleChanged {
            occurred_at: DateTime::from_timestamp(0, 0).unwrap(),
            org_id: Uuid::nil(),
            user: UserPayload {
                id: Uuid::nil(),
                email: "sam@acme.test".into(),
                name: "Sam".into(),
                org_role: "manager".into(),
                method: None,
            },
            previous_role: "member".into(),
        };
        let json = event.to_json();
        assert_eq!(event.hook_name(), "user_role_changed");
        assert!(json.contains("\"previous_role\":\"member\""));
        // The login-only `method` field is omitted for admin user events.
        assert!(!json.contains("\"method\""));
    }

    #[test]
    fn active_transition_fires_only_on_a_real_flip() {
        use ActiveTransition::{Deactivated, Reactivated};
        assert_eq!(active_transition(Some(true), false), Some(Deactivated));
        assert_eq!(active_transition(Some(false), true), Some(Reactivated));
        // No-op sets (same value) emit nothing (FR-012).
        assert_eq!(active_transition(Some(true), true), None);
        assert_eq!(active_transition(Some(false), false), None);
        // Absent prior row → no event.
        assert_eq!(active_transition(None, true), None);
        assert_eq!(active_transition(None, false), None);
    }
}
