# Tasks: Time Tracking & Invoicing

**Feature**: `001-time-tracking-invoicing` | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

**Format**: `- [ ] [ID] [P?] [Story?] Description with file path`

- `[P]` = can run in parallel (different files, no dependency on an incomplete task).
- `[USn]` = belongs to User Story n (Setup/Foundational/Polish carry no story label).

**Context**: Much of P1–P4 already exists in the codebase; those tasks say "complete/reconcile". P3 invoicing and P5 plugins are largely new. Correctness paths (duration/money/totals) live in `crates/core`. Tests follow the repo conventions: pure unit tests in `crates/core`, `#[sqlx::test]` integration tests in `crates/horae/tests/`, and the `nix flake check` e2e VM test.

______________________________________________________________________

## Phase 1: Setup (shared infrastructure — mostly in place)

- [x] T001 Verify the workspace builds and the dev environment is reproducible: `nix develop`, `cargo build -p horae --features server`, `dx serve` renders (per [quickstart.md](./quickstart.md)).
- [x] T002 [P] Confirm formatting/lint gates pass locally: `nix fmt -- --ci` and `cargo clippy -p horae --features server`.
- [x] T003 Confirm a PostgreSQL target is reachable and `horae migrate run` + `horae seed` succeed (`crates/horae/src/db.rs`, `crates/horae/src/seed.rs`).

______________________________________________________________________

## Phase 2: Foundational (blocking prerequisites for all stories)

**⚠️ Complete before starting any user story.**

- [x] T004 Reconcile the SQL schema in `crates/horae/migrations/0001_init.sql` with [data-model.md](./data-model.md) (organizations, users, clients, projects, tasks + project-task membership, assignments, time_entries; enums `org_role`/`project_type`/`budget_kind`/`entry_state`/`round_dir`).
- [x] T005 [P] Verify exact-value domain logic in `crates/core/src/{duration,money,rounding,totals}.rs` (integer minutes, integer minor units + ISO currency) and the entry/invoice transitions in `crates/core/src/state.rs` (FR-023).
- [x] T006 [P] Unit tests for core correctness in `crates/core/src/{duration,rounding,totals}.rs` (`#[cfg(test)]`): totals equal the exact sum of parts across groupings (SC-002).
- [x] T007 Confirm shared server state and pool wiring in `crates/horae/src/state.rs` (`AppState` in `OnceCell`) and `crates/horae/src/db.rs` (pool + eager migrations on `serve`).
- [x] T008 [P] Confirm session/auth foundation and role model in `crates/horae/src/auth/` (session store, OIDC, `DEV_LOGIN` bypass) and the `org_role` authorization helper used by server functions (FR-001).
- [x] T009 [P] Confirm error/logging foundation in `crates/horae/src/error.rs` and tracing init (`crates/horae/src/main.rs`).

______________________________________________________________________

## Phase 3: User Story 1 — Track billable time (Priority: P1) 🎯 MVP

**Goal**: A signed-in user can start/stop a timer and add/edit manual entries against a project/task, with exact durations and correct totals.

**Independent test**: With seeded project/task, start→stop a timer, add a manual entry, edit an entry, see a correct day total — no client/invoice setup beyond seed.

- [x] T010 [P] [US1] Confirm the `TimeEntry` model and mapping in `crates/horae/src/models/time_entry.rs` matches data-model (user/project/task, `started_at`/`ended_at`/`duration_minutes`, `notes`, `billable`, `entry_state`, `invoice_id`).
- [x] T011 [US1] Complete the time-entry server functions in `crates/horae/src/server_fns.rs`: `list_time_entries` (with filters), `create_time_entry`, `update_time_entry`, `delete_time_entry`, `start_timer`, `stop_timer` (see [contracts/server-functions.md](./contracts/server-functions.md)).
- [x] T012 [US1] Enforce a single running timer per user in `start_timer` (`crates/horae/src/server_fns.rs`) — reject or auto-stop the prior running entry (FR-004, AS5).
- [x] T013 [US1] Compute duration exactly on stop via `crates/core` from `started_at`/`ended_at` in `stop_timer` (`crates/horae/src/server_fns.rs`) (FR-003, FR-023).
- [x] T014 [US1] Block edit/delete of entries already `invoiced` in `update_time_entry`/`delete_time_entry` (`crates/horae/src/server_fns.rs`) (FR-015).
- [x] T015 [P] [US1] Timer widget live-increment in `crates/horae/src/components/timer_widget.rs` and the time page list/add/edit UI in `crates/horae/src/pages/time.rs`.
- [x] T016 [P] [US1] Dashboard summary (current-period hours + running timer) in `crates/horae/src/pages/dashboard.rs` (FR-017).
- [x] T017 [US1] Integration tests in `crates/horae/tests/integration.rs` (`#[sqlx::test]`): start/stop produces exact duration; second concurrent timer prevented; invoiced entry is immutable.

**Checkpoint**: US1 is an independently shippable MVP.

______________________________________________________________________

## Phase 4: User Story 2 — Organize clients, projects, tasks (Priority: P2)

**Goal**: Managers manage clients, projects (billing method/budget/rate), and tasks; inactive items drop out of new-entry pickers.

**Independent test**: Create client → project → task; the task becomes selectable when logging time; deactivating a project hides it from pickers.

- [x] T018 [P] [US2] Confirm `Client`/`Project`/`Task` models in `crates/horae/src/models/{client,project,task}.rs` against data-model (billing method, budget, currency, active flags).
- [x] T019 [US2] Add missing management server functions in `crates/horae/src/server_fns.rs` (contracts mark these **planned**): `update_client`/`deactivate_client`, `update_project`/`deactivate_project`, `create_task`/`update_task`/`deactivate_task`.
- [x] T020 [US2] Reconcile authorization: gate client/project/task management at **manager** (per FR-008/009/010), correcting the current admin-only gating flagged in [contracts/server-functions.md](./contracts/server-functions.md).
- [x] T021 [US2] Exclude inactive clients/projects/tasks from new-entry pickers while preserving links on existing entries (`crates/horae/src/server_fns.rs` list queries + `crates/horae/src/pages/{clients,projects}.rs`) (FR-011).
- [x] T022 [P] [US2] Client/project/task management UI in `crates/horae/src/pages/clients.rs` and `crates/horae/src/pages/projects.rs`.
- [x] T023 [US2] Integration tests in `crates/horae/tests/integration.rs`: new task becomes loggable; inactive project hidden from pickers but retained on history.

______________________________________________________________________

## Phase 5: User Story 3 — Invoice tracked time (Priority: P3)

**Goal**: Managers generate an invoice for a client from billable, un-invoiced time; totals reconcile exactly; lifecycle draft→sent→paid/void; export.

**Independent test**: With billable time, generate a draft invoice for a period; line items/total match the entries; mark sent→paid; export matches on-screen.

- [x] T024 [US3] Add invoices schema in `crates/horae/migrations/` (new migration): `invoices` (+ status `draft|sent|paid|void`, number, issue/due dates, currency, total_cents) and invoice line items referencing time (data-model.md). *(invoices table does not exist yet)*
- [x] T025 [P] [US3] `Invoice` (+ line item) model in `crates/horae/src/models/invoice.rs`.
- [x] T026 [US3] `generate_invoice(client_id, period)` server function in `crates/horae/src/server_fns.rs`: select billable, un-invoiced entries; build lines and an exact total via `crates/core`; mark entries `invoiced` (FR-012/FR-013/FR-023).
- [x] T026a [US3] Implement FR-024 rate resolution (task → assignment override → project → user default) in the invoice-line computation in `crates/core` and `crates/horae/src/server_fns.rs`.
- [x] T027 [US3] `list_invoices` (replace the current empty stub) and `get_invoice` in `crates/horae/src/server_fns.rs` (contracts note the stub).
- [x] T028 [US3] `update_invoice_status` in `crates/horae/src/server_fns.rs` enforcing the invoice state machine, including `void` returning entries to un-invoiced (data-model.md state machine, FR-014).
- [x] T029 [US3] Prevent double-billing and editing invoiced time at the query/mutation layer (`crates/horae/src/server_fns.rs`) (FR-013/FR-015).
- [x] T030 [P] [US3] Invoice export handler in `crates/horae/src/reports.rs` (reuse CSV/XLSX; add invoice document) so exported amounts match on-screen (FR-016/SC-007).
- [x] T030a [US3] Add the Typst rendering toolchain (Cargo dep or `typst` CLI) and wire nixpkgs fonts into the dev shell / package for reproducible typography (`crates/horae/Cargo.toml`, `nix/`).
- [x] T030b [P] [US3] Author the invoice Typst template in `crates/horae/templates/invoice.typ` (branding, line items, totals, provider/bank details).
- [x] T030c [US3] Implement deterministic invoice PDF rendering (invoice data + branding → Typst → PDF) in `crates/horae/src/reports.rs` / a render module, reconciling exactly (FR-025/SC-007); see [contracts/invoice-rendering.md](./contracts/invoice-rendering.md).
- [x] T030d [P] [US3] Editable-fields review UI before finalize/send (provider identity, bank details, notes) in `crates/horae/src/pages/invoices.rs` (FR-025).
- [x] T030e [US3] Store per-organization invoice-branding settings (provider identity, bank details, logo, default template) in `crates/horae/migrations/` + `crates/horae/src/models/organization.rs`.
- [x] T031 [P] [US3] Invoice list + detail UI in `crates/horae/src/pages/invoices.rs`.
- [x] T032 [US3] Integration tests in `crates/horae/tests/integration.rs`: generated invoice total equals summed entries; entries become invoiced and cannot be re-billed; void restores entries.

______________________________________________________________________

## Phase 6: User Story 4 — Administer users & access (Priority: P4)

**Goal**: Admins create users, assign roles, and deactivate accounts; roles gate actions.

**Independent test**: Create a member and a manager; verify the member cannot manage clients/invoices; deactivating a user blocks sign-in and preserves history.

- [ ] T033 [US4] Implement the user-management server functions in `crates/horae/src/server_fns.rs` and/or `crates/horae/src/cli.rs`: `create_user`, `set_user_role`, `deactivate_user`, `list_users` — replacing the current CLI stubs flagged in [contracts/cli.md](./contracts/cli.md).
- [ ] T034 [US4] Enforce that deactivated users cannot authenticate in `crates/horae/src/auth/` (session/OIDC/dev paths) (FR-002).
- [ ] T035 [P] [US4] Admin users UI in `crates/horae/src/pages/admin.rs`.
- [ ] T036 [US4] Integration tests in `crates/horae/tests/integration.rs`: role gating (member denied manager actions); deactivated user cannot sign in; history retained.

______________________________________________________________________

## Phase 7: User Story 5 — Extend with plugins (Priority: P5)

**Goal**: Operator-installed sandboxed WASM plugins react to events and contribute dashboard widgets, isolated from the core. *(Entirely new — see [contracts/plugin-interface.md](./contracts/plugin-interface.md).)*

**Independent test**: Install a plugin subscribed to `invoice_sent`; sending an invoice fires it within ~1s; a failing/slow plugin never blocks the core action; a widget-returning plugin renders on the dashboard.

- [ ] T037 [US5] Add the `extism` dependency (server feature) in `crates/horae/Cargo.toml`.
- [ ] T038 [P] [US5] `plugin.toml` manifest parsing in `crates/horae/src/plugin/manifest.rs` (name, version, hooks).
- [ ] T039 [P] [US5] `AppEvent` enum + JSON payloads in `crates/horae/src/plugin/event.rs` (time_entry_created/stopped, invoice_created/sent, user_logged_in).
- [x] T040 [P] [US5] extism host functions in `crates/horae/src/plugin/host.rs`: `horae_log`, read-only `horae_db_query`, `horae_http_post`, `horae_config_get` (FR-020).
- [ ] T041 [US5] `PluginRegistry` in `crates/horae/src/plugin/registry.rs`: scan `{dataDir}/plugins/`, load `*.wasm`, index by subscribed hook (FR-018); expose `dispatch(event)` with concurrent, time-bounded, failure-isolated calls (FR-021/SC-006).
- [ ] T042 [US5] Hold `plugins: Arc<PluginRegistry>` in `AppState` (`crates/horae/src/state.rs`) and load at startup (`crates/horae/src/main.rs`).
- [ ] T043 [US5] Dispatch events after the relevant DB writes in `crates/horae/src/server_fns.rs` (time entry created/stopped, invoice created/sent, user signed in) (FR-019).
- [ ] T044 [P] [US5] Dashboard-widget contract + rendering of plugin-returned widgets in `crates/horae/src/pages/dashboard.rs` / a plugin slot component (FR-022).
- [ ] T045 [US5] Integration test in `crates/horae/tests/integration.rs`: a sample plugin receives a dispatched event; a failing plugin does not break the triggering action.

______________________________________________________________________

## Phase 8: Polish & cross-cutting concerns

- [ ] T046 [P] Reconcile authorization consistently across all server functions with the spec's role model (member/manager/admin), resolving the mismatches noted in the contracts.
- [ ] T047 [P] Ensure the read-only Harvest API (`crates/horae/src/harvest/`) reflects finalized entities (e.g. real client join for `report_time` grouping) per [contracts/harvest-api.md](./contracts/harvest-api.md).
- [ ] T048 Validate list/report performance at target scale (≥50 users / 100k entries) returns < 2s (SC-005); add indexes in `crates/horae/migrations/` if needed.
- [ ] T049 [P] Update `README.md`/`AGENTS.md` for any new commands, the invoices flow, and the plugin system.
- [ ] T050 Run the full [quickstart.md](./quickstart.md) validation and `nix flake check` (formatting + e2e VM); confirm all Success Criteria (SC-001..SC-008).

______________________________________________________________________

## Dependencies & execution order

- **Setup (Ph1) → Foundational (Ph2)** gate everything.
- **User stories** are prioritized P1→P5 and each is independently testable once Ph2 is done. Recommended order matches priority; US2 (structure) makes US1 richer, and US3 (invoicing) depends on US1+US2 data existing.
- **US5 (plugins)** depends only on the events emitted by US1/US3/US4 code paths (T043) but its scaffolding (T037–T042) is independent and can start any time after Ph2.
- **Polish (Ph8)** last.

## Parallel opportunities

- Ph2: T005/T006/T008/T009 in parallel `[P]`.
- Within a story, `[P]` model/UI tasks run alongside each other (e.g. US1: T015/T016; US3: T025/T030/T031; US5: T038/T039/T040/T044).
- Different user stories can be developed by different people in parallel after Ph2, since each targets a distinct slice.

## Implementation strategy

- **MVP = User Story 1** (Phase 3): shippable time tracking on its own.
- Then layer P2 (structure) → P3 (invoicing) → P4 (admin) → P5 (plugins), each an independently demoable increment.
- Tests are included per story because correctness/exactness is a hard requirement (SC-002/SC-007); they follow the repo's `crates/core` unit + `#[sqlx::test]` integration + `nix flake check` e2e conventions.
