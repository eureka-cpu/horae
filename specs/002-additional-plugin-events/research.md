# Research: Additional Plugin Events

Phase 0 decisions. The feature builds on an existing, well-understood subsystem (the feature
`001` plugin system), so there are no open `NEEDS CLARIFICATION` items; the work here is
choosing *how* to extend it consistently.

## 1. Where the budget-crossing decision lives

- **Decision**: A pure function in `horae-core` (`budget.rs`), e.g. `crossed_band(consumed, budget, &thresholds, last_band) -> Option<u8>`, operating entirely on integers (minutes or cents) and returning the newly crossed threshold percentage, if any.
- **Rationale**: Constitution I (exactness) and II (domain purity) — budget comparisons are correctness-critical and must be integer-only and unit-testable in isolation. Keeping the percentage math out of SQL and out of the `horae` crate makes it verifiable.
- **Alternatives considered**: Inline the arithmetic in `server_fns.rs` (rejected — not unit-testable, tempts float division); compute the percentage in SQL (rejected — spreads correctness-critical math into the query layer).

## 2. De-duplicating budget threshold events

- **Decision**: Persist the highest threshold band already announced for each project (`projects.last_budget_alert_pct`, nullable). Fire `project_budget_threshold_reached` only when consumption crosses into a *higher* configured band than the stored one; fire `project_over_budget` when it first exceeds 100%. Reset the marker if consumption drops below a band (e.g., entries deleted/voided).
- **Rationale**: SC-004 requires exactly one event per crossing and none on continued rises within a band. A stored band is O(1) to check and survives restarts.
- **Alternatives considered**: Recompute "was it already over?" from history on every entry (rejected — costly and racy under concurrency); in-memory dedupe (rejected — lost on restart, would re-announce).

## 3. The `timer_running_too_long` scheduler

- **Decision**: A server-only `tokio` background task spawned at `serve` startup, polling every 60s for `is_running` entries whose `started_at` is older than the configured limit and that have not yet been marked notified. It dispatches one event per such timer and sets `time_entries.notified_long_running_at`. The marker is cleared when the timer stops (in `stop_timer`).
- **Rationale**: This event is time-based, not caused by any mutation, so it cannot ride the post-write dispatch pattern used by every other event. A coarse poll with a persisted marker gives "exactly once per overrun" (SC-001) without spamming on each tick.
- **Alternatives considered**: Fire from within `get_current_timer`/list calls (rejected — depends on someone loading the page, so a truly forgotten timer might never notify); no marker, fire every tick (rejected — violates "exactly one"); an external cron (rejected — adds an operational dependency the base app avoids).

## 4. Configuration surface for thresholds and the long-timer limit

- **Decision**: Organization-level settings — `budget_alert_pcts int[]` (default `{80,100}`) and `long_timer_minutes int` (default `480`) — added to the org/branding settings row (single-org today, `org_id`-scoped for the future).
- **Rationale**: The spec (FR-015) requires configurable, admin-level thresholds shared across plugins, not per-plugin parameters. An org-level setting keeps every subscribed plugin consistent.
- **Alternatives considered**: Per-plugin config in `plugin.toml` (rejected — different plugins would see different thresholds for the same project); hard-coded constants (rejected — FR-015 requires configurability).

## 5. Suppressing no-op events (FR-012, SC-002)

- **Decision**: For `update_*`, dispatch only when the write actually changed a field; for `set_*_active`, compare the incoming flag to the current value and emit `*_deactivated`/`*_reactivated` only on a real flip. Use the row returned by the `UPDATE ... RETURNING` plus a guarded `WHERE` (or a pre-read) to detect change.
- **Rationale**: Events are past-tense facts; a re-save with identical data is not a business event.
- **Alternatives considered**: Always emit on every call (rejected — violates FR-012 and creates plugin noise); diff every column client-side (rejected — server is the source of truth).

## 6. Event definition & payload style

- **Decision**: Follow the existing `AppEvent` pattern exactly — a serde-tagged enum variant per event (`#[serde(rename = "hook_name")]`), a `hook_name()` arm, and small `*Payload` structs mirroring model fields. `user_role_changed` carries `previous_role` alongside the new role.
- **Rationale**: Consistency with the shipped catalog (FR-010/FR-014) means zero interface change and no new serialization concept for plugin authors.
- **Alternatives considered**: A generic "entity changed" event with a type discriminator (rejected — loses the past-tense, self-describing hook names plugins subscribe to).

## 7. Dispatch call-site placement

- **Decision**: Reuse `state.plugins.dispatch(event)` at the same point the existing five events use — immediately after the DB write commits inside each `#[server]` function (and in `auth` for `user_logged_out`).
- **Rationale**: Preserves the failure-isolation guarantee (FR-011): the mutation is already committed, and dispatch spawns tasks and returns immediately.
- **Alternatives considered**: A DB trigger / outbox (rejected — moves logic out of the single server-function mutation path, against Constitution IV, and is more machinery than needed here).
