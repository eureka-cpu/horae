# Plugin Event Catalog — Addendum

**Extends:** the Event catalog in
[`../../001-time-tracking-invoicing/contracts/plugin-interface.md`](../../001-time-tracking-invoicing/contracts/plugin-interface.md)
(FR-018–FR-022). This document adds events only; the manifest, envelope, host functions,
dashboard-widget contract, and sandbox guarantees are unchanged.

## Envelope (unchanged)

Every event is a JSON object with the shared envelope plus one payload key:

```json
{ "event": "<hook_name>", "occurred_at": "2026-07-13T10:00:00Z", "org_id": "<uuid>", "<payload_key>": { } }
```

A plugin subscribes by listing the hook name in `plugin.toml` `hooks` and exporting a WASM
function of the same name. Dispatch remains non-blocking and failure-isolated: the triggering
mutation is committed before any plugin runs, and a slow/failing plugin never affects it.

## A. Direct events

Each fires immediately after the named server mutation commits. No event fires on a no-op
(unchanged update, or `set_active` called with the same value).

| Hook | Trigger | Payload key |
|---|---|---|
| `time_entry_updated` | `update_time_entry` | `time_entry` |
| `time_entry_deleted` | `delete_time_entry` | `time_entry` |
| `timesheet_submitted` | `submit_week` | `submission` |
| `submission_approved` | `approve_submission` | `submission` |
| `submission_rejected` | `reject_submission` | `submission` |
| `invoice_paid` | `update_invoice_status` → `paid` | `invoice` |
| `invoice_voided` | `update_invoice_status` → `void` | `invoice` |
| `client_created` / `client_updated` | `create_client` / `update_client` | `client` |
| `client_deactivated` / `client_reactivated` | `set_client_active` (on flip) | `client` |
| `project_created` / `project_updated` | `create_project` / `update_project` | `project` |
| `project_deactivated` / `project_reactivated` | `set_project_active` (on flip) | `project` |
| `task_created` / `task_updated` | `create_task` / `update_task` | `task` |
| `user_created` | `create_user` | `user` |
| `user_role_changed` | `set_user_role` | `user` + `previous_role` |
| `user_deactivated` | `set_user_active` → inactive | `user` |
| `user_logged_out` | `logout` | `user` |
| `user_assigned_to_project` | `create_assignment` | `assignment` |
| `assignment_removed` | `delete_assignment` | `assignment` |
| `org_branding_updated` | `update_org_branding` | `org` |

Representative payloads (the `time_entry`, `invoice`, and `user` payloads are unchanged from
the base catalog):

```json
{
  "event": "user_role_changed",
  "occurred_at": "2026-07-13T10:00:00Z",
  "org_id": "0195...-0001",
  "previous_role": "member",
  "user": { "id": "0195...-0002", "email": "sam@acme.test", "name": "Sam", "org_role": "manager", "active": true }
}
```

```json
{
  "event": "timesheet_submitted",
  "occurred_at": "2026-07-13T10:00:00Z",
  "org_id": "0195...-0001",
  "submission": { "id": "0195...-00a1", "user_id": "0195...-0002", "week_start": "2026-07-06", "status": "submitted", "total_minutes": 2280 }
}
```

```json
{
  "event": "project_deactivated",
  "occurred_at": "2026-07-13T10:00:00Z",
  "org_id": "0195...-0001",
  "project": { "id": "0195...-0005", "client_id": "0195...-0003", "name": "Website", "project_type": "time_and_materials", "budget_kind": "amount", "active": false }
}
```

## B. Derived events

Computed from state, not emitted verbatim after one write.

| Hook | Fires when | Payload key |
|---|---|---|
| `project_budget_threshold_reached` | consumption crosses a configured band (default 80%) | `budget` |
| `project_over_budget` | consumption first exceeds 100% of budget | `budget` |
| `timer_running_too_long` | a running timer exceeds the configured limit (default 8h) | `time_entry` + `running_minutes` |

Each band fires **at most once** per crossing (deduped via `projects.last_budget_alert_pct`);
`timer_running_too_long` fires **at most once** per overrun (deduped via
`time_entries.notified_long_running_at`, cleared on stop). Budget events never fire for projects
whose `budget_kind` is `none`.

```json
{
  "event": "project_budget_threshold_reached",
  "occurred_at": "2026-07-13T10:00:00Z",
  "org_id": "0195...-0001",
  "budget": {
    "project": { "id": "0195...-0005", "client_id": "0195...-0003", "name": "Website", "project_type": "time_and_materials", "budget_kind": "amount", "active": true },
    "threshold_pct": 80,
    "consumed_minutes": 4800,
    "consumed_cents": 800000,
    "budget_minutes": null,
    "budget_amount_cents": 1000000
  }
}
```

```json
{
  "event": "timer_running_too_long",
  "occurred_at": "2026-07-13T10:00:00Z",
  "org_id": "0195...-0001",
  "running_minutes": 495,
  "time_entry": { "id": "0195...-0abc", "user_id": "0195...-0002", "project_id": "0195...-0005", "task_id": "0195...-0007", "spent_date": "2026-07-13", "minutes": 0, "billable": true, "is_running": true, "started_at": "2026-07-13T01:45:00Z" }
}
```

## Configuration

Two organization-level settings govern the derived events (admin-configurable, shared across
all plugins — not per-plugin): `budget_alert_pcts` (default `[80, 100]`) and `long_timer_minutes`
(default `480`). See [../data-model.md](../data-model.md).

## Compatibility

The five existing events (`time_entry_created`, `time_entry_stopped`, `invoice_created`,
`invoice_sent`, `user_logged_in`) are unchanged in name, envelope, and payload. Existing plugins
require no changes; new events are opt-in via `hooks`.
