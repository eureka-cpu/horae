//! Report and plugin-widget server functions.

use super::*;

// ── Reports (M8) ────────────────────────────────────────────────────────────

/// Grouped time report. Groups by "project", "task", "client", or "person", with
/// optional client/project/teammate filters. Each group carries billable and cost
/// amounts (rates via FR-024); its `currency` is `None` when it mixes currencies.
#[server]
pub async fn report_time(
    from: String,
    to: String,
    group_by: String,
    client_id: Option<String>,
    project_id: Option<String>,
    user_id: Option<String>,
) -> Result<Vec<ReportRow>, ServerFnError> {
    let _user_id = session_user_id().await?;
    let state = crate::state::global_state().await;

    let from_date: chrono::NaiveDate = from
        .parse()
        .map_err(|_| server_err("Invalid from date (use YYYY-MM-DD)"))?;
    let to_date: chrono::NaiveDate = to
        .parse()
        .map_err(|_| server_err("Invalid to date (use YYYY-MM-DD)"))?;
    let client_filter = parse_opt_uuid(client_id, "client_id")?;
    let project_filter = parse_opt_uuid(project_id, "project_id")?;
    let user_filter = parse_opt_uuid(user_id, "user_id")?;

    let rows = sqlx::query_as!(
        TimeRow,
        r#"SELECT
             p.name AS project_name, t.name AS task_name, u.name AS user_name,
             c.name AS client_name, c.currency AS currency,
             te.minutes, te.rounded_minutes, te.billable,
             pt.rate_cents AS task_rate_cents,
             a.rate_cents AS assignment_rate_cents,
             u.billable_rate_cents AS user_billable_rate_cents,
             u.cost_rate_cents AS user_cost_rate_cents
           FROM time_entries te
           JOIN projects p ON te.project_id = p.id
           JOIN clients c ON p.client_id = c.id
           JOIN tasks t ON te.task_id = t.id
           JOIN users u ON te.user_id = u.id
           LEFT JOIN project_tasks pt ON pt.project_id = te.project_id AND pt.task_id = te.task_id
           LEFT JOIN assignments a ON a.project_id = te.project_id AND a.user_id = te.user_id
           WHERE te.spent_date BETWEEN $1 AND $2
             AND ($3::uuid IS NULL OR p.client_id = $3)
             AND ($4::uuid IS NULL OR te.project_id = $4)
             AND ($5::uuid IS NULL OR te.user_id = $5)"#,
        from_date as chrono::NaiveDate,
        to_date as chrono::NaiveDate,
        client_filter,
        project_filter,
        user_filter,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(aggregate_time(&rows, &group_by))
}

/// One time entry joined with the names, currency, and rates the report groups by.
#[cfg(feature = "server")]
struct TimeRow {
    project_name: String,
    task_name: String,
    user_name: String,
    client_name: String,
    currency: String,
    minutes: i32,
    rounded_minutes: Option<i32>,
    billable: bool,
    task_rate_cents: Option<i64>,
    assignment_rate_cents: Option<i64>,
    user_billable_rate_cents: Option<i64>,
    user_cost_rate_cents: Option<i64>,
}

/// Group entries by the requested dimension, summing hours and money. Billable
/// amount uses the FR-024 rate cascade so a report agrees with invoicing; cost
/// uses the user's cost rate. A group's `currency` is `None` once it spans two
/// currencies, since the amounts are then no longer summable.
#[cfg(feature = "server")]
fn aggregate_time(rows: &[TimeRow], group_by: &str) -> Vec<ReportRow> {
    #[derive(Default)]
    struct Agg {
        total: i64,
        rounded: i64,
        billable_min: i64,
        billable_cents: i64,
        cost_cents: i64,
        currency: Option<String>,
        mixed: bool,
    }

    let mut groups: std::collections::BTreeMap<String, Agg> = std::collections::BTreeMap::new();
    for r in rows {
        let label = match group_by {
            "task" => r.task_name.clone(),
            "client" => r.client_name.clone(),
            "person" => r.user_name.clone(),
            _ => r.project_name.clone(),
        };
        let rounded = r.rounded_minutes.unwrap_or(r.minutes) as i64;
        let g = groups.entry(label).or_default();
        g.total += r.minutes as i64;
        g.rounded += rounded;
        if r.billable {
            g.billable_min += rounded;
            let rate = horae_core::invoice::resolve_rate(
                r.task_rate_cents,
                r.assignment_rate_cents,
                r.user_billable_rate_cents,
            )
            .unwrap_or(0);
            g.billable_cents += horae_core::invoice::line_amount_cents(rate, r.minutes);
        }
        g.cost_cents +=
            horae_core::invoice::line_amount_cents(r.user_cost_rate_cents.unwrap_or(0), r.minutes);
        if !g.mixed {
            match &g.currency {
                None => g.currency = Some(r.currency.clone()),
                Some(cur) if cur != &r.currency => {
                    g.mixed = true;
                    g.currency = None;
                }
                _ => {}
            }
        }
    }

    groups
        .into_iter()
        .map(|(label, g)| ReportRow {
            label,
            total_minutes: g.total,
            rounded_minutes: g.rounded,
            billable_minutes: g.billable_min,
            billable_cents: g.billable_cents,
            cost_cents: g.cost_cents,
            currency: g.currency,
        })
        .collect()
}

/// Detailed (per-entry) report for the range, with the same optional filters.
#[server]
pub async fn report_detailed(
    from: String,
    to: String,
    client_id: Option<String>,
    project_id: Option<String>,
    user_id: Option<String>,
) -> Result<Vec<DetailedReportRow>, ServerFnError> {
    let _user_id = session_user_id().await?;
    let state = crate::state::global_state().await;

    let from_date: chrono::NaiveDate = from
        .parse()
        .map_err(|_| server_err("Invalid from date (use YYYY-MM-DD)"))?;
    let to_date: chrono::NaiveDate = to
        .parse()
        .map_err(|_| server_err("Invalid to date (use YYYY-MM-DD)"))?;
    let client_filter = parse_opt_uuid(client_id, "client_id")?;
    let project_filter = parse_opt_uuid(project_id, "project_id")?;
    let user_filter = parse_opt_uuid(user_id, "user_id")?;

    let entries = sqlx::query_as!(
        DetailedReportRow,
        r#"SELECT te.spent_date as "spent_date: chrono::NaiveDate",
                p.name AS project_name, t.name AS task_name,
                u.name AS user_name, te.minutes, te.rounded_minutes, te.billable, te.notes
         FROM time_entries te
         JOIN projects p ON te.project_id = p.id
         JOIN tasks t ON te.task_id = t.id
         JOIN users u ON te.user_id = u.id
         WHERE te.spent_date BETWEEN $1 AND $2
           AND ($3::uuid IS NULL OR p.client_id = $3)
           AND ($4::uuid IS NULL OR te.project_id = $4)
           AND ($5::uuid IS NULL OR te.user_id = $5)
         ORDER BY te.spent_date, p.name, t.name"#,
        from_date as chrono::NaiveDate,
        to_date as chrono::NaiveDate,
        client_filter,
        project_filter,
        user_filter,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    Ok(entries)
}

// ── Plugins ────────────────────────────────────────────────────────────────

/// Collect dashboard widgets from all loaded plugins (FR-022).
#[server]
pub async fn get_plugin_widgets() -> Result<Vec<PluginWidget>, ServerFnError> {
    let state = crate::state::global_state().await;
    let widgets = state.plugins.collect_widgets().await;
    Ok(widgets
        .into_iter()
        .map(|w| PluginWidget {
            plugin_name: w.plugin_name,
            title: w.title,
            body: w.body,
        })
        .collect())
}

/// A dashboard widget contributed by a plugin, serializable for the SPA.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginWidget {
    pub plugin_name: String,
    pub title: String,
    pub body: String,
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    fn trow(
        currency: &str,
        minutes: i32,
        billable: bool,
        task_rate: Option<i64>,
        user_bill: Option<i64>,
        user_cost: Option<i64>,
    ) -> TimeRow {
        TimeRow {
            project_name: "P".into(),
            task_name: "T".into(),
            user_name: "U".into(),
            client_name: "C".into(),
            currency: currency.into(),
            minutes,
            rounded_minutes: None,
            billable,
            task_rate_cents: task_rate,
            assignment_rate_cents: None,
            user_billable_rate_cents: user_bill,
            user_cost_rate_cents: user_cost,
        }
    }

    #[test]
    fn billable_uses_the_rate_cascade_and_cost_uses_the_user_rate() {
        // 1h billable: task rate (10000/h) wins the cascade over the user rate;
        // cost is the user's cost rate (6000/h).
        let out = aggregate_time(
            &[trow("EUR", 60, true, Some(10000), Some(9999), Some(6000))],
            "project",
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].billable_cents, 10000);
        assert_eq!(out[0].cost_cents, 6000);
        assert_eq!(out[0].currency.as_deref(), Some("EUR"));
    }

    #[test]
    fn non_billable_time_has_cost_but_no_billable_amount() {
        let out = aggregate_time(&[trow("EUR", 60, false, None, None, Some(6000))], "project");
        assert_eq!(out[0].billable_cents, 0);
        assert_eq!(out[0].cost_cents, 6000);
    }

    #[test]
    fn a_group_spanning_two_currencies_reports_no_currency() {
        // Grouped by person, one teammate's entries land in one group; the two
        // client currencies make the group's money non-summable.
        let out = aggregate_time(
            &[
                trow("EUR", 60, true, Some(10000), None, None),
                trow("USD", 60, true, Some(10000), None, None),
            ],
            "person",
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].currency, None);
    }
}
