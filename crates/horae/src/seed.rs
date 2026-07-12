/// Seed the database with a demo organisation, users, clients, projects, tasks,
/// and sample time entries covering the current ISO week (Mon–Fri).
///
/// All INSERTs use ON CONFLICT DO NOTHING so this is safe to run multiple times.
use chrono::{Datelike, NaiveDate, Utc};
use horae_core::types::{BudgetKind, OrgRole, ProjectRole, ProjectType, RoundDir};
use sqlx::PgPool;
use uuid::Uuid;

// ── Fixed UUIDs for idempotent seeding ───────────────────────────────────────

#[allow(clippy::unusual_byte_groupings)]
const ORG_ID: Uuid = Uuid::from_u128(0x0195_0000_0000_7000_8000_000000000001);
#[allow(clippy::unusual_byte_groupings)]
const ADMIN_ID: Uuid = Uuid::from_u128(0x0195_0000_0000_7000_8000_000000000002);
#[allow(clippy::unusual_byte_groupings)]
const CLIENT_ACME_ID: Uuid = Uuid::from_u128(0x0195_0000_0000_7000_8000_000000000003);
#[allow(clippy::unusual_byte_groupings)]
const CLIENT_TECH_ID: Uuid = Uuid::from_u128(0x0195_0000_0000_7000_8000_000000000004);
#[allow(clippy::unusual_byte_groupings)]
const PROJ_ACME_ID: Uuid = Uuid::from_u128(0x0195_0000_0000_7000_8000_000000000005);
#[allow(clippy::unusual_byte_groupings)]
const PROJ_TECH_ID: Uuid = Uuid::from_u128(0x0195_0000_0000_7000_8000_000000000006);
#[allow(clippy::unusual_byte_groupings)]
const TASK_DEV_ID: Uuid = Uuid::from_u128(0x0195_0000_0000_7000_8000_000000000007);
#[allow(clippy::unusual_byte_groupings)]
const TASK_DESIGN_ID: Uuid = Uuid::from_u128(0x0195_0000_0000_7000_8000_000000000008);
#[allow(clippy::unusual_byte_groupings)]
const TASK_MEETING_ID: Uuid = Uuid::from_u128(0x0195_0000_0000_7000_8000_000000000009);
#[allow(clippy::unusual_byte_groupings)]
const TASK_REVIEW_ID: Uuid = Uuid::from_u128(0x0195_0000_0000_7000_8000_00000000000a);

pub async fn run(pool: &PgPool) -> anyhow::Result<()> {
    tracing::info!("Seeding demo data…");

    // Organisation
    sqlx::query!(
        "INSERT INTO organizations (id, name, default_currency, week_start, round_minutes, round_dir)
         VALUES ($1, $2, 'EUR', 1, 15, $3)
         ON CONFLICT (id) DO NOTHING",
        ORG_ID,
        "Demo Org",
        RoundDir::Nearest as RoundDir,
    )
    .execute(pool)
    .await?;

    // Admin user (used for DEV_LOGIN)
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role, billable_rate_cents)
         VALUES ($1, $2, 'admin@example.com', 'Admin User', $3, 10000)
         ON CONFLICT (id) DO NOTHING",
        ADMIN_ID,
        ORG_ID,
        OrgRole::Admin as OrgRole,
    )
    .execute(pool)
    .await?;

    // Clients
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency)
         VALUES ($1, $2, 'Acme Corp', 'EUR')
         ON CONFLICT (id) DO NOTHING",
        CLIENT_ACME_ID,
        ORG_ID,
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency)
         VALUES ($1, $2, 'TechStart Inc', 'USD')
         ON CONFLICT (id) DO NOTHING",
        CLIENT_TECH_ID,
        ORG_ID,
    )
    .execute(pool)
    .await?;

    // Projects
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, code, name, project_type, currency, budget_kind, budget_minutes)
         VALUES ($1, $2, $3, 'ACME-01', 'Acme Website Redesign', $4, 'EUR', $5, 12000)
         ON CONFLICT (id) DO NOTHING",
        PROJ_ACME_ID,
        ORG_ID,
        CLIENT_ACME_ID,
        ProjectType::TimeAndMaterials as ProjectType,
        BudgetKind::Hours as BudgetKind,
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, code, name, project_type, currency, budget_kind, budget_amount_cents)
         VALUES ($1, $2, $3, 'TECH-01', 'TechStart API Integration', $4, 'USD', $5, 1500000)
         ON CONFLICT (id) DO NOTHING",
        PROJ_TECH_ID,
        ORG_ID,
        CLIENT_TECH_ID,
        ProjectType::FixedFee as ProjectType,
        BudgetKind::Amount as BudgetKind,
    )
    .execute(pool)
    .await?;

    // Tasks (org-level catalog)
    for (id, name, billable, rate_cents) in [
        (TASK_DEV_ID, "Development", true, 12000i64),
        (TASK_DESIGN_ID, "Design", true, 10000i64),
        (TASK_MEETING_ID, "Meetings", false, 8000i64),
        (TASK_REVIEW_ID, "Code Review", true, 11000i64),
    ] {
        sqlx::query!(
            "INSERT INTO tasks (id, org_id, name, billable_default, default_rate_cents)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (id) DO NOTHING",
            id,
            ORG_ID,
            name,
            billable,
            rate_cents,
        )
        .execute(pool)
        .await?;
    }

    // Enable tasks on each project (rate_cents overrides the cascade for billing)
    for (proj_id, task_id, billable, rate_cents) in [
        (PROJ_ACME_ID, TASK_DEV_ID, true, Some(12000i64)),
        (PROJ_ACME_ID, TASK_DESIGN_ID, true, Some(10000i64)),
        (PROJ_ACME_ID, TASK_MEETING_ID, false, None),
        (PROJ_TECH_ID, TASK_DEV_ID, true, Some(15000i64)),
        (PROJ_TECH_ID, TASK_REVIEW_ID, true, Some(11000i64)),
        (PROJ_TECH_ID, TASK_MEETING_ID, false, None),
    ] {
        sqlx::query!(
            "INSERT INTO project_tasks (project_id, task_id, billable, rate_cents)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (project_id, task_id) DO UPDATE
             SET rate_cents = EXCLUDED.rate_cents",
            proj_id,
            task_id,
            billable,
            rate_cents,
        )
        .execute(pool)
        .await?;
    }

    // Assign admin to both projects
    for proj_id in [PROJ_ACME_ID, PROJ_TECH_ID] {
        sqlx::query!(
            "INSERT INTO assignments (id, project_id, user_id, role, rate_cents)
             VALUES (gen_random_uuid(), $1, $2, $3, 12000)
             ON CONFLICT (project_id, user_id) DO NOTHING",
            proj_id,
            ADMIN_ID,
            ProjectRole::Lead as ProjectRole,
        )
        .execute(pool)
        .await?;
    }

    // Sample time entries — Mon–Fri of the current ISO week
    let today = Utc::now().date_naive();
    let monday = iso_week_monday(today);

    let entries: &[(NaiveDate, Uuid, Uuid, i32, &str, bool)] = &[
        // Mon
        (
            monday,
            PROJ_ACME_ID,
            TASK_DEV_ID,
            150,
            "Set up project scaffolding",
            true,
        ),
        (
            monday,
            PROJ_ACME_ID,
            TASK_MEETING_ID,
            60,
            "Kickoff call with client",
            false,
        ),
        // Tue
        (
            monday + days(1),
            PROJ_ACME_ID,
            TASK_DESIGN_ID,
            210,
            "Homepage wireframes",
            true,
        ),
        (
            monday + days(1),
            PROJ_TECH_ID,
            TASK_MEETING_ID,
            45,
            "Sprint planning",
            false,
        ),
        // Wed
        (
            monday + days(2),
            PROJ_TECH_ID,
            TASK_DEV_ID,
            300,
            "Auth middleware",
            true,
        ),
        (
            monday + days(2),
            PROJ_TECH_ID,
            TASK_REVIEW_ID,
            90,
            "Review PR #12",
            true,
        ),
        // Thu
        (
            monday + days(3),
            PROJ_ACME_ID,
            TASK_DEV_ID,
            240,
            "CMS integration",
            true,
        ),
        (
            monday + days(3),
            PROJ_TECH_ID,
            TASK_DEV_ID,
            180,
            "Webhook endpoint",
            true,
        ),
        // Fri
        (
            monday + days(4),
            PROJ_ACME_ID,
            TASK_DESIGN_ID,
            120,
            "Component library",
            true,
        ),
        (
            monday + days(4),
            PROJ_TECH_ID,
            TASK_REVIEW_ID,
            60,
            "Review & merge",
            true,
        ),
    ];

    for (date, project_id, task_id, minutes, notes, billable) in entries {
        sqlx::query!(
            "INSERT INTO time_entries
               (id, org_id, user_id, project_id, task_id, spent_date, minutes, notes, billable)
             VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT DO NOTHING",
            ORG_ID,
            ADMIN_ID,
            *project_id,
            *task_id,
            *date as chrono::NaiveDate,
            *minutes,
            *notes,
            *billable,
        )
        .execute(pool)
        .await?;
    }

    tracing::info!("Seed complete.");
    Ok(())
}

/// Returns the Monday of the ISO week containing `date`.
fn iso_week_monday(date: NaiveDate) -> NaiveDate {
    let days_since_monday = date.weekday().num_days_from_monday();
    date - chrono::Duration::days(days_since_monday as i64)
}

fn days(n: i64) -> chrono::Duration {
    chrono::Duration::days(n)
}

/// Verify that the seed data looks reasonable (called after seeding).
pub async fn verify(pool: &PgPool) -> anyhow::Result<()> {
    let entry_count = sqlx::query_scalar!("SELECT COUNT(*) FROM time_entries")
        .fetch_one(pool)
        .await?
        .unwrap_or(0);
    tracing::info!("time_entries: {entry_count} rows");

    let client_count = sqlx::query_scalar!("SELECT COUNT(*) FROM clients")
        .fetch_one(pool)
        .await?
        .unwrap_or(0);
    tracing::info!("clients: {client_count} rows");

    Ok(())
}
