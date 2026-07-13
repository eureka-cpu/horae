# Quickstart: Additional Plugin Events

Validation scenarios that prove the addendum works end-to-end. Assumes the feature `001` plugin
system is present and the tasks in [tasks.md](./tasks.md) are implemented. Event names and
payloads are defined in [contracts/plugin-events-addendum.md](./contracts/plugin-events-addendum.md).

## Prerequisites

- Dev shell (`nix develop`) with a PostgreSQL reachable via `DATABASE_URL`.
- Migrations applied and seed data loaded: `cargo run -p horae --features server -- migrate run` then `… seed`.
- The reusable fixture plugins from feature `001` (`crates/horae/tests/fixtures/plugins/echo-plugin`, `fail-plugin`).

## Scenario 1 — Direct event delivery (US1/US2)

1. Point a test plugin's `plugin.toml` at the new hooks, e.g. `hooks = ["invoice_paid", "timesheet_submitted", "user_role_changed"]`.
1. Trigger each action through its server function: mark an invoice paid, submit a week, change a user's role.
1. **Expected**: the plugin receives exactly one event per action; `user_role_changed` includes `previous_role`; no other events arrive.

Automated equivalent: `#[sqlx::test]` cases assert that each mutation dispatches its event once
(and that the payload matches the contract).

## Scenario 2 — No-op suppression (FR-012 / SC-002)

1. Subscribe to `project_updated` and `project_deactivated`.
1. Call `update_project` with values identical to the current row; call `set_project_active(id, true)` on an already-active project.
1. **Expected**: **zero** events — no event fires when nothing changed.

## Scenario 3 — Budget threshold & over-budget (US3)

1. Configure a project with an `amount` budget and org `budget_alert_pcts = {80,100}`.
1. Log billable time until consumption reaches ~82% of budget.
1. **Expected**: one `project_budget_threshold_reached` with `threshold_pct = 80`.
1. Log more time within the same band. **Expected**: no further event.
1. Push consumption past 100%. **Expected**: one `project_over_budget`.

Automated equivalent: `horae-core` unit tests for `budget::crossed_band` (integer inputs,
boundary cases) plus an integration test that a time-entry write crosses a band exactly once.

## Scenario 4 — Long-running timer (US3, scheduler)

1. Set org `long_timer_minutes` low for the test (e.g., 1) and start a timer.
1. Let the scheduler tick.
1. **Expected**: one `timer_running_too_long` with `running_minutes` past the limit; no repeat on the next tick; stopping the timer clears the marker.

## Scenario 5 — Failure isolation (FR-011 / SC-003)

1. Subscribe the `fail-plugin` to `invoice_paid` (it errors/hangs).
1. Mark an invoice paid.
1. **Expected**: the invoice status change succeeds with no added latency; the failing plugin does not affect it.

## Gate

- `cargo test -p horae-core` (budget math) and `DATABASE_URL=… cargo test -p horae --features server` (dispatch + no-op) green.
- `nix fmt -- --ci` and `nix flake check` green; `.sqlx` cache regenerated after new queries.
- The five pre-existing events still behave identically (SC-005).
