//! Report and plugin-widget server functions.

use super::*;

// ── Reports (M8) ────────────────────────────────────────────────────────────

/// Grouped time report. Groups by "project", "task", "client", or "person".
#[server]
pub async fn report_time(
    from: String,
    to: String,
    group_by: String,
) -> Result<Vec<ReportRow>, ServerFnError> {
    let _user_id = session_user_id().await?;
    let state = crate::state::global_state().await;

    let from_date: chrono::NaiveDate = from
        .parse()
        .map_err(|_| server_err("Invalid from date (use YYYY-MM-DD)"))?;
    let to_date: chrono::NaiveDate = to
        .parse()
        .map_err(|_| server_err("Invalid to date (use YYYY-MM-DD)"))?;

    // Fetch detailed rows and group in Rust for flexibility
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
         ORDER BY te.spent_date"#,
        from_date as chrono::NaiveDate,
        to_date as chrono::NaiveDate,
    )
    .fetch_all(&state.db)
    .await
    .map_err(server_err)?;

    // Group by the requested dimension
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, (i64, i64, i64)> = BTreeMap::new();
    for e in &entries {
        let label = match group_by.as_str() {
            "task" => e.task_name.clone(),
            "client" => e.project_name.clone(), // project serves as proxy until we join clients
            "person" => e.user_name.clone(),
            _ => e.project_name.clone(), // default: "project"
        };
        let agg = groups.entry(label).or_insert((0, 0, 0));
        agg.0 += e.minutes as i64;
        agg.1 += e.rounded_minutes.unwrap_or(e.minutes) as i64;
        if e.billable {
            agg.2 += e.rounded_minutes.unwrap_or(e.minutes) as i64;
        }
    }

    let rows: Vec<ReportRow> = groups
        .into_iter()
        .map(|(label, (total, rounded, billable))| ReportRow {
            label,
            total_minutes: total,
            rounded_minutes: rounded,
            billable_minutes: billable,
        })
        .collect();

    Ok(rows)
}

/// Detailed (per-entry) report for the given date range.
#[server]
pub async fn report_detailed(
    from: String,
    to: String,
) -> Result<Vec<DetailedReportRow>, ServerFnError> {
    let _user_id = session_user_id().await?;
    let state = crate::state::global_state().await;

    let from_date: chrono::NaiveDate = from
        .parse()
        .map_err(|_| server_err("Invalid from date (use YYYY-MM-DD)"))?;
    let to_date: chrono::NaiveDate = to
        .parse()
        .map_err(|_| server_err("Invalid to date (use YYYY-MM-DD)"))?;

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
         ORDER BY te.spent_date, p.name, t.name"#,
        from_date as chrono::NaiveDate,
        to_date as chrono::NaiveDate,
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
