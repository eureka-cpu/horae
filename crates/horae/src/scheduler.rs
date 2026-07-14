//! Periodic detection of forgotten (long-running) timers.
//!
//! Unlike every other plugin event, `timer_running_too_long` is time-based:
//! nothing mutates when a timer simply keeps running, so it is found by polling
//! rather than by a post-write dispatch. Each overrun is announced at most once,
//! guarded by `time_entries.notified_long_running_at` (cleared on stop).

use crate::state::AppState;

const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

/// Spawn the background poller. Call once at server startup.
pub fn spawn(state: &'static AppState) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(POLL_INTERVAL);
        loop {
            ticker.tick().await;
            if let Err(e) = sweep(state).await {
                tracing::warn!("long-timer scheduler tick failed: {e}");
            }
        }
    });
}

/// One poll: announce every running timer past its org's limit that has not been
/// announced yet, and mark it so it is not announced again until it stops.
async fn sweep(state: &AppState) -> anyhow::Result<()> {
    let rows = sqlx::query!(
        r#"SELECT te.id, te.org_id, te.user_id, te.project_id, te.task_id,
                  te.spent_date as "spent_date: chrono::NaiveDate",
                  te.minutes, te.billable, te.notes,
                  te.started_at as "started_at!: chrono::DateTime<chrono::Utc>",
                  (EXTRACT(EPOCH FROM (now() - te.started_at)) / 60)::int as "running_minutes!"
           FROM time_entries te
           JOIN organizations o ON o.id = te.org_id
           WHERE te.is_running = true
             AND te.notified_long_running_at IS NULL
             AND te.started_at < now() - make_interval(mins => o.long_timer_minutes)"#,
    )
    .fetch_all(&state.db)
    .await?;

    for r in rows {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::TimerRunningTooLong {
                occurred_at: chrono::Utc::now(),
                org_id: r.org_id,
                running_minutes: r.running_minutes,
                time_entry: crate::plugin::event::TimeEntryPayload {
                    id: r.id,
                    user_id: r.user_id,
                    project_id: r.project_id,
                    task_id: r.task_id,
                    spent_date: r.spent_date,
                    minutes: r.minutes,
                    billable: r.billable,
                    is_running: true,
                    notes: r.notes,
                    started_at: Some(r.started_at),
                },
            });
        sqlx::query!(
            "UPDATE time_entries SET notified_long_running_at = now() WHERE id = $1",
            r.id,
        )
        .execute(&state.db)
        .await?;
    }
    Ok(())
}
