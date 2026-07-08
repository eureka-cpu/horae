use serde::Serialize;
use std::collections::HashMap;

// ── Pagination envelope ─────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HarvestPagination<T: Serialize> {
    #[serde(flatten)]
    pub data: HashMap<String, Vec<T>>,
    pub per_page: i64,
    pub total_pages: i64,
    pub total_entries: i64,
    pub page: i64,
    pub next_page: Option<i64>,
    pub previous_page: Option<i64>,
    pub links: HarvestLinks,
}

impl<T: Serialize> HarvestPagination<T> {
    pub fn new(
        key: &str,
        items: Vec<T>,
        page: i64,
        per_page: i64,
        total_entries: i64,
        base_url: &str,
    ) -> Self {
        let total_pages = if total_entries == 0 {
            1
        } else {
            (total_entries + per_page - 1) / per_page
        };
        let next_page = if page < total_pages {
            Some(page + 1)
        } else {
            None
        };
        let previous_page = if page > 1 { Some(page - 1) } else { None };

        let links = HarvestLinks {
            first: format!("{}?page=1&per_page={}", base_url, per_page),
            next: next_page.map(|p| format!("{}?page={}&per_page={}", base_url, p, per_page)),
            previous: previous_page
                .map(|p| format!("{}?page={}&per_page={}", base_url, p, per_page)),
            last: format!("{}?page={}&per_page={}", base_url, total_pages, per_page),
        };

        let mut data = HashMap::new();
        data.insert(key.to_string(), items);

        Self {
            data,
            per_page,
            total_pages,
            total_entries,
            page,
            next_page,
            previous_page,
            links,
        }
    }
}

#[derive(Serialize)]
pub struct HarvestLinks {
    pub first: String,
    pub next: Option<String>,
    pub previous: Option<String>,
    pub last: String,
}

// ── Resource DTOs ───────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct HarvestRef {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Clone)]
pub struct HarvestProjectRef {
    pub id: String,
    pub name: String,
    pub code: Option<String>,
}

// ── Time Entry ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HarvestTimeEntry {
    pub id: String,
    pub spent_date: String,
    pub hours: f64,
    pub rounded_hours: f64,
    pub notes: Option<String>,
    pub is_locked: bool,
    pub locked_reason: Option<String>,
    pub is_closed: bool,
    pub is_billed: bool,
    pub is_running: bool,
    pub timer_started_at: Option<String>,
    pub billable: bool,
    pub budgeted: bool,
    pub billable_rate: Option<f64>,
    pub cost_rate: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
    pub user: HarvestRef,
    pub client: HarvestRef,
    pub project: HarvestProjectRef,
    pub task: HarvestRef,
    pub approval_status: String,
}

// ── Project ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HarvestProject {
    pub id: String,
    pub name: String,
    pub code: Option<String>,
    pub is_active: bool,
    pub is_billable: bool,
    pub bill_by: String,
    pub budget_by: String,
    pub budget: Option<f64>,
    pub starts_on: Option<String>,
    pub ends_on: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub client: HarvestRef,
}

// ── Client ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HarvestClient {
    pub id: String,
    pub name: String,
    pub is_active: bool,
    pub address: Option<String>,
    pub currency: String,
    pub created_at: String,
    pub updated_at: String,
}

// ── Task ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HarvestTask {
    pub id: String,
    pub name: String,
    pub is_active: bool,
    pub billable_by_default: bool,
    pub default_hourly_rate: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
}

// ── User ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct HarvestUser {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    pub is_active: bool,
    pub is_admin: bool,
    pub cost_rate: Option<f64>,
    pub default_hourly_rate: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
}
