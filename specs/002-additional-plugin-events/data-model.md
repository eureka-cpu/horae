# Data Model: Additional Plugin Events

This feature adds **event types** (in-memory, serialized to plugins) and a small amount of
**persisted state** (config + dedupe bookkeeping). It does not introduce new core business
entities. Payload field shapes are the contract; see
[contracts/plugin-events-addendum.md](./contracts/plugin-events-addendum.md) for the JSON.

## Event types (in `crates/horae/src/plugin/event.rs`)

New `AppEvent` variants, each carrying the common envelope fields (`occurred_at`, `org_id`) plus
one payload. Hook name = the serde tag.

| Variant / hook | Payload | Notes |
|---|---|---|
| `time_entry_updated` | `TimeEntryPayload` (existing) | reused |
| `time_entry_deleted` | `TimeEntryPayload` | id + user/project/task; `is_running` false |
| `timesheet_submitted` | `SubmissionPayload` | |
| `submission_approved` | `SubmissionPayload` | |
| `submission_rejected` | `SubmissionPayload` | |
| `invoice_paid` | `InvoicePayload` (existing) | reused |
| `invoice_voided` | `InvoicePayload` | reused |
| `client_created` / `client_updated` | `ClientPayload` | |
| `client_deactivated` / `client_reactivated` | `ClientPayload` | only on active flip |
| `project_created` / `project_updated` | `ProjectPayload` | |
| `project_deactivated` / `project_reactivated` | `ProjectPayload` | only on active flip |
| `task_created` / `task_updated` | `TaskPayload` | |
| `task_deactivated` / `task_reactivated` | `TaskPayload` | only on active flip |
| `user_created` | `UserPayload` (existing) | reused |
| `user_role_changed` | `UserPayload` + `previous_role: String` | carries prior role |
| `user_deactivated` | `UserPayload` | only on active flip to inactive |
| `user_logged_out` | `UserPayload` | dispatched from `auth` |
| `user_assigned_to_project` / `assignment_removed` | `AssignmentPayload` | |
| `org_branding_updated` | `OrgBrandingPayload` | subset of branding fields |
| `project_budget_threshold_reached` | `BudgetThresholdPayload` | derived; one per band crossed |
| `project_over_budget` | `BudgetThresholdPayload` | derived; `threshold_pct = 100` |
| `timer_running_too_long` | `TimeEntryPayload` + `running_minutes: i32` | derived; from scheduler |

### New payload structs

- **`SubmissionPayload`**: `id: Uuid`, `user_id: Uuid`, `week_start: NaiveDate`, `status: String`, `total_minutes: i32`.
- **`ClientPayload`**: `id: Uuid`, `name: String`, `currency: String`, `active: bool`.
- **`ProjectPayload`**: `id: Uuid`, `client_id: Uuid`, `name: String`, `project_type: String`, `budget_kind: String`, `active: bool`.
- **`TaskPayload`**: `id: Uuid`, `name: String`, `billable_default: bool`, `default_rate_cents: Option<i64>`, `active: bool`.
- **`AssignmentPayload`**: `id: Uuid`, `project_id: Uuid`, `user_id: Uuid`, `role: String`.
- **`OrgBrandingPayload`**: `org_id: Uuid`, plus the branding fields that changed (identity/bank/logo presence — no secrets beyond what the UI already exposes).
- **`BudgetThresholdPayload`**: `project: ProjectPayload`, `threshold_pct: i16`, `consumed_minutes: i32`, `consumed_cents: i64`, `budget_minutes: Option<i64>`, `budget_amount_cents: Option<i64>`.

All payload numeric fields are **integers** (minutes, cents) — never floats (Constitution I).

## Persisted state (migration)

Additions only; no table rewrites.

### Organization settings (org-level config, FR-015)

| Column | Type | Default | Purpose |
|---|---|---|---|
| `budget_alert_pcts` | `int[]` | `{80,100}` | threshold bands that fire `project_budget_threshold_reached` |
| `long_timer_minutes` | `int` | `480` | limit for `timer_running_too_long` |

Stored on the existing organization/branding settings row (single-org now; `org_id`-scoped).
Values are validated: percentages in `1..=100`, ascending, unique; `long_timer_minutes > 0`.

### Project dedupe marker (budget events)

| Column | Type | Default | Purpose |
|---|---|---|---|
| `projects.last_budget_alert_pct` | `smallint` | `NULL` | highest band already announced; prevents duplicate crossings (research §2) |

### Time-entry dedupe marker (long-timer event)

| Column | Type | Default | Purpose |
|---|---|---|---|
| `time_entries.notified_long_running_at` | `timestamptz` | `NULL` | set when `timer_running_too_long` fired; cleared on stop; prevents re-notification (research §3) |

## Domain logic (in `crates/horae-core`)

- **`budget::crossed_band(consumed, budget, thresholds, last_band) -> Option<u8>`** — pure,
  integer-only. Given consumed vs budget (minutes for `hours` budgets, cents for `amount`
  budgets), the configured bands, and the last announced band, returns the newly crossed band
  or `None`. Unit-tested against exactness (Constitution I).

## Validation & state rules

- **No-op suppression** (FR-012): `update_*` events fire only when a field actually changed;
  `*_deactivated`/`*_reactivated` and `user_deactivated` fire only when `active` flips.
- **Threshold monotonicity**: a band fires at most once while consumption stays at/above it;
  `last_budget_alert_pct` advances upward and is reset downward when consumption falls below a
  band (e.g., entries removed or an invoice voided).
- **Long-timer once-per-overrun**: an entry emits `timer_running_too_long` at most once;
  `notified_long_running_at` gates re-emission and is cleared when the timer stops.
- **Budget kind**: `project_over_budget` / threshold events apply only to projects whose
  `budget_kind` is `hours` or `amount`; `none` projects never trigger budget events.
