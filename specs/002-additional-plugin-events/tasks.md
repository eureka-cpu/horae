# Tasks: Additional Plugin Events

**Input**: Design documents from `/specs/002-additional-plugin-events/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/plugin-events-addendum.md

**Tests**: Included — this feature is correctness-critical (integer budget math) and the spec's
success criteria require verifying exact/one-time delivery and no-op suppression.

**Dependency**: Builds on the feature `001` / User Story 5 plugin subsystem (`AppEvent`,
`PluginRegistry`, `dispatch`, fixture plugins). That work must be merged first (PR #40).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1 / US2 / US3

______________________________________________________________________

## Phase 1: Setup (Shared Infrastructure)

- [x] T001 Confirm the US5 plugin subsystem is present and builds: `cargo build -p horae --features server` compiles `crates/horae/src/plugin/` (`AppEvent`, `PluginRegistry`, `dispatch`) and the fixture plugins under `crates/horae/tests/fixtures/plugins/` load.
- [x] T002 [P] Confirm the offline query cache workflow: `SQLX_OFFLINE=true` build is green and `.sqlx/` is current (baseline before adding queries).

______________________________________________________________________

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared test scaffolding every story relies on.

- [ ] T003 Add an integration-test helper in `crates/horae/tests/integration.rs` that drives a `PluginRegistry` with a recording fixture plugin and asserts "exactly one event of hook X" (and "zero events") for a given action, reusing the feature-001 registry harness.

**Checkpoint**: Foundation ready — user stories can proceed (in priority order or in parallel).

______________________________________________________________________

## Phase 3: User Story 1 — Time & invoicing lifecycle events (Priority: P1) 🎯 MVP

**Goal**: Emit `time_entry_updated`/`deleted`, `timesheet_submitted`/`submission_approved`/`rejected`, `invoice_paid`/`voided`, reusing existing payloads, with no-op suppression.

**Independent Test**: Subscribe a fixture plugin to these hooks, drive each action, confirm exactly one event per real change and none on a no-op update.

### Tests for User Story 1

- [ ] T004 [US1] Integration tests in `crates/horae/tests/integration.rs`: editing an entry emits one `time_entry_updated`; deleting emits one `time_entry_deleted`; an update with identical values emits none (FR-012).
- [ ] T005 [US1] Integration tests in `crates/horae/tests/integration.rs`: submit → approve → reject each emit one `timesheet_submitted` / `submission_approved` / `submission_rejected` in order.
- [ ] T006 [US1] Integration tests in `crates/horae/tests/integration.rs`: status change to `paid` emits `invoice_paid`; to `void` emits `invoice_voided`; other transitions emit neither.

### Implementation for User Story 1

- [x] T007 [US1] Add the US1 `AppEvent` variants and `hook_name()` arms plus `SubmissionPayload` in `crates/horae/src/plugin/event.rs` (`time_entry_updated`, `time_entry_deleted`, `timesheet_submitted`, `submission_approved`, `submission_rejected`, `invoice_paid`, `invoice_voided`).
- [x] T008 [US1] Dispatch `time_entry_updated` after a real change in `update_time_entry`, and `time_entry_deleted` in `delete_time_entry`, in `crates/horae/src/server_fns.rs` (suppress on no-op update).
- [x] T009 [US1] Dispatch `timesheet_submitted` in `submit_week` in `crates/horae/src/server_fns.rs`.
- [x] T010 [US1] Dispatch `submission_approved` in `approve_submission` and `submission_rejected` in `reject_submission` in `crates/horae/src/server_fns.rs`.
- [x] T011 [US1] Dispatch `invoice_paid` / `invoice_voided` on the matching status transitions in `update_invoice_status` in `crates/horae/src/server_fns.rs`.
- [x] T012 [US1] Regenerate the `.sqlx/` cache for any changed queries and run the US1 tests green.

**Checkpoint**: US1 fully functional and independently testable.

______________________________________________________________________

## Phase 4: User Story 2 — Administrative & catalog events (Priority: P2)

**Goal**: Emit user, client, project, task, assignment, and org-branding events, with `*_deactivated`/`*_reactivated` only on a real active-flag flip and `user_role_changed` carrying the previous role.

**Independent Test**: Subscribe a fixture plugin to these hooks; change a role, toggle a project's active flag twice, add/remove an assignment; confirm one event per real change, correct `previous_role`, and no event on unchanged saves.

### Tests for User Story 2

- [ ] T013 [P] [US2] Integration tests in `crates/horae/tests/integration.rs`: client/project/task create & update emit one event each; `set_*_active` emits `*_deactivated`/`*_reactivated` only when the flag flips (none on same-value).
- [ ] T014 [P] [US2] Integration tests in `crates/horae/tests/integration.rs`: `create_user` emits `user_created`; `set_user_role` emits `user_role_changed` with `previous_role`; `set_user_active`→inactive emits `user_deactivated`.
- [ ] T015 [P] [US2] Integration tests in `crates/horae/tests/integration.rs`: `create_assignment`/`delete_assignment` emit `user_assigned_to_project`/`assignment_removed`; `update_org_branding` emits `org_branding_updated`.

### Implementation for User Story 2

- [x] T016 [US2] Add the US2 `AppEvent` variants + `hook_name()` arms and `ClientPayload`/`ProjectPayload`/`TaskPayload`/`AssignmentPayload`/`OrgBrandingPayload` (and `previous_role`) in `crates/horae/src/plugin/event.rs`.
- [x] T017 [US2] Dispatch client events in `create_client`/`update_client`/`set_client_active` in `crates/horae/src/server_fns.rs` (no-op + flip suppression).
- [x] T018 [US2] Dispatch project events in `create_project`/`update_project`/`set_project_active` in `crates/horae/src/server_fns.rs`.
- [x] T019 [US2] Dispatch task events in `create_task`/`update_task`/`set_task_active` in `crates/horae/src/server_fns.rs` (`task_deactivated`/`task_reactivated` only on active flip).
- [x] T020 [US2] Dispatch `user_created` in `create_user`, `user_role_changed` (reading the prior role before the update) in `set_user_role`, and `user_deactivated` in `set_user_active` in `crates/horae/src/server_fns.rs`.
- [x] T021 [US2] Dispatch `user_logged_out` in the logout path in `crates/horae/src/auth/` (mirroring where `user_logged_in` is dispatched).
- [x] T022 [US2] Dispatch `user_assigned_to_project`/`assignment_removed` in `create_assignment`/`delete_assignment` in `crates/horae/src/server_fns.rs`.
- [x] T023 [US2] Dispatch `org_branding_updated` in `update_org_branding` in `crates/horae/src/server_fns.rs`.
- [x] T024 [US2] Regenerate the `.sqlx/` cache and run the US2 tests green.

**Checkpoint**: US1 and US2 both work independently.

______________________________________________________________________

## Phase 5: User Story 3 — Budget & long-timer derived events (Priority: P3)

**Goal**: Emit `project_budget_threshold_reached`/`project_over_budget` (computed, integer-only, deduped per band) and `timer_running_too_long` (periodic scheduler, once per overrun), with org-level config.

**Independent Test**: Configure a project budget and org thresholds; log time across 80% and 100% and confirm one event per crossing; set a low long-timer limit and confirm one `timer_running_too_long` per overrun.

### Tests for User Story 3

- [x] T025 [P] [US3] Unit tests for `budget::crossed_band` in `crates/core/src/budget.rs` (`#[cfg(test)]`): integer inputs, band boundaries, no re-fire within a band, reset when consumption drops (SC-004, Constitution I).
- [ ] T026 [US3] Integration tests in `crates/horae/tests/integration.rs`: crossing 80% fires one `project_budget_threshold_reached`; staying in band fires none; exceeding 100% fires `project_over_budget`; a `none`-budget project fires nothing.
- [ ] T027 [US3] Integration test in `crates/horae/tests/integration.rs`: an entry older than `long_timer_minutes` is detected once, sets `notified_long_running_at`, and is not re-detected; stopping clears the marker.

### Implementation for User Story 3

- [x] T028 [US3] Migration `crates/horae/migrations/NNNN_plugin_event_support.sql`: add org `budget_alert_pcts int[] default '{80,100}'` and `long_timer_minutes int default 480`; `projects.last_budget_alert_pct smallint`; `time_entries.notified_long_running_at timestamptz`.
- [x] T029 [P] [US3] Implement the pure `budget::crossed_band(consumed, budget, thresholds, last_band)` function in `crates/core/src/budget.rs` and export it from `crates/core/src/lib.rs` (integer-only).
- [x] T030 [US3] Add the US3 `AppEvent` variants + `hook_name()` arms and `BudgetThresholdPayload` in `crates/horae/src/plugin/event.rs`.
- [x] T031 [US3] After time-entry writes (`create_time_entry`, `update_time_entry`, `delete_time_entry`, `stop_timer`) in `crates/horae/src/server_fns.rs`, recompute project consumption, call `budget::crossed_band`, dispatch `project_budget_threshold_reached`/`project_over_budget`, and update/reset `projects.last_budget_alert_pct`.
- [x] T032 [US3] Implement the periodic long-running-timer scheduler in `crates/horae/src/scheduler.rs` (server-only `tokio` task): poll for `is_running` entries past `long_timer_minutes` without `notified_long_running_at`, dispatch `timer_running_too_long`, set the marker.
- [x] T033 [US3] Clear `notified_long_running_at` in `stop_timer` (`crates/horae/src/server_fns.rs`).
- [x] T034 [US3] Read org config and spawn the scheduler at startup: wire it in `crates/horae/src/state.rs` / `crates/horae/src/main.rs` (`serve`).
- [x] T035 [US3] Regenerate the `.sqlx/` cache for the new queries and run the US3 tests green.

**Checkpoint**: All three stories independently functional.

______________________________________________________________________

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T036 [P] Extend the base event catalog in `specs/001-time-tracking-invoicing/contracts/plugin-interface.md` (or reference this addendum) so the full event list is discoverable in one place.
- [ ] T037 [P] Update `crates/horae/tests/fixtures/plugins/` (or add one) so a fixture subscribes to a new hook, and add a failure-isolation test: a `fail-plugin` on `invoice_paid` does not affect the status change (FR-011 / SC-003).
- [ ] T038 Verify the five pre-existing events are unchanged in name/envelope/payload (SC-005) — a regression assertion in `crates/horae/tests/integration.rs`.
- [x] T039 Run `nix fmt -- --ci` and `nix flake check`; confirm `.sqlx/` committed and green.
- [ ] T040 Run the [quickstart.md](./quickstart.md) scenarios end-to-end and confirm all success criteria (SC-001..SC-006).

______________________________________________________________________

## Dependencies & Execution Order

- **Setup (Phase 1)** → **Foundational (Phase 2)** → **User Stories (Phase 3–5)** → **Polish (Phase 6)**.
- **US1 (P1)** and **US2 (P2)** need **no schema change** (they reuse existing data) and are independent of each other and of US3.
- **US3 (P3)** owns the migration (T028), the `horae-core` budget logic (T029), and the scheduler — the only net-new schema and background component.
- Within a story: event definitions (`event.rs`) and tests can precede dispatch wiring; `.sqlx` regeneration comes after query changes.

## Parallel Opportunities

- T002 (setup) is [P].
- Across stories: once Foundational (T003) is done, US1, US2, and US3 can be built in parallel by different people; US3's `budget.rs` (T029) is [P] since it touches only `crates/core`.
- The US2 test tasks (T013–T015) are [P] with each other only if split into distinct test modules/files; if all land in `integration.rs`, sequence them.

## Implementation Strategy

- **MVP** = Phase 1 + 2 + **US1** (the money-and-time lifecycle events), then stop and validate.
- **Incremental**: add US2 (admin/catalog), then US3 (derived budget/timer), each independently testable and shippable.
- Merge the US5 base (PR #40) before starting; regenerate `.sqlx` and keep `nix flake check` green at every checkpoint.
