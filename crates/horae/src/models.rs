pub mod approval;
pub mod assignment;
pub mod client;
pub mod invoice;
pub mod organization;
pub mod project;
pub mod task;
pub mod time_entry;
pub mod user;

pub use approval::{Approval, ApprovalSummary};
pub use assignment::Assignment;
pub use client::Client;
pub use invoice::{Invoice, InvoiceLine, InvoiceWithLines};
pub use organization::OrgBranding;
pub use project::Project;
pub use task::Task;
pub use time_entry::TimeEntry;
pub use user::User;

/// Per-project tracked totals: all logged minutes, and the billable amount in
/// cents (rates resolved via FR-024). Powers the Projects overview's Spent column.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectSpend {
    pub project_id: uuid::Uuid,
    pub spent_minutes: i64,
    pub spent_cents: i64,
}

// ── Report DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportRow {
    pub label: String,
    pub total_minutes: i64,
    pub rounded_minutes: i64,
    pub billable_minutes: i64,
    /// Billable amount in cents (rates resolved via FR-024) and cost in cents
    /// (`users.cost_rate_cents`). `currency` is the ISO code these amounts are in,
    /// or `None` when the group mixes clients of different currencies — in which
    /// case the money is not summable and the UI shows it as unavailable.
    pub billable_cents: i64,
    pub cost_cents: i64,
    pub currency: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct DetailedReportRow {
    pub spent_date: chrono::NaiveDate,
    pub project_name: String,
    pub task_name: String,
    pub user_name: String,
    pub minutes: i32,
    pub rounded_minutes: Option<i32>,
    pub billable: bool,
    pub notes: Option<String>,
}
