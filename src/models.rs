pub mod approval;
pub mod assignment;
pub mod client;
pub mod invoice;
pub mod organization;
pub mod project;
pub mod task;
pub mod time_entry;
pub mod user;

pub use approval::Approval;
pub use assignment::Assignment;
pub use client::Client;
pub use invoice::Invoice;
pub use organization::Organization;
pub use project::Project;
pub use task::Task;
pub use time_entry::TimeEntry;
pub use user::User;

// ── Report DTOs ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportRow {
    pub label: String,
    pub total_minutes: i64,
    pub rounded_minutes: i64,
    pub billable_minutes: i64,
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
