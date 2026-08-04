# Tasks: Timed Calendar Entries

**Input**: Design documents from `specs/003-timed-calendar-entries/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Included — the Constitution requires `horae-core` unit tests and `#[sqlx::test]` integration tests, and quickstart.md lists them as gates.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no incomplete dependency)
- **[Story]**: US1 drag-create · US2 positioning/timer · US3 untimed compat · US4 resize/move

## Path Conventions

Two-crate layout from plan.md: pure domain in `crates/core/src/`, app (server + web) in `crates/horae/src/`, migrations in `crates/horae/migrations/`, integration tests in `crates/horae/tests/`.

______________________________________________________________________

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Schema and offline-cache groundwork every story builds on.

- [X] T001 Add migration `crates/horae/migrations/0005_time_entry_start_minute.sql` adding nullable `start_minute smallint` with the range and within-day CHECK constraints per [data-model.md](./data-model.md).
- [X] T002 Apply the migration to the dev DB and regenerate the committed `.sqlx/` cache (`cargo sqlx prepare --workspace -- --features server --all-targets`).

______________________________________________________________________

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Domain helpers, the DTO field, the shared mutation plumbing, and calendar positioning that ALL stories depend on. No user story is complete until this phase is.

- [X] T003 [P] Create pure module `crates/core/src/time_of_day.rs` with `parse`, `format`, `format_12h`, `snap(minutes, step)`, `clamp_to_day(start, minutes)`, and `MIN_DURATION` per [data-model.md](./data-model.md); export it from `crates/core/src/lib.rs`.
- [X] T004 [P] Unit tests for `time_of_day` in `crates/core/src/time_of_day.rs` (parse valid/invalid, format round-trip, snap to 15, clamp at end-of-day, min-duration) — `cargo test -p horae-core`.
- [X] T005 [P] Add `start_minute: Option<i32>` to the shared `TimeEntry` DTO in `crates/horae/src/models/time_entry.rs`.
- [X] T006 Include `start_minute` in the read queries/mappings (`list_time_entries` and any `query_as!`) in `crates/horae/src/server_fns/time_entries.rs` so the calendar receives it.
- [X] T007 Extend `create_time_entry` and `update_time_entry` in `crates/horae/src/server_fns/time_entries.rs` to accept and persist `start_minute` (snap + `clamp_to_day` via `horae-core`; reject out-of-range) per [contracts/server-fns.md](./contracts/server-fns.md).
- [X] T008 In `render_calendar_view` (`crates/horae/src/pages/timesheet.rs`), split each day's entries into timed (positioned at `start_minute`) and untimed (stacked from the top as today), sizing both by duration; add supporting CSS in `crates/horae/assets/css/horae.css` (D5).

**Checkpoint**: entries can carry a start time end-to-end and the calendar renders timed vs untimed correctly — user-story phases can now proceed.

______________________________________________________________________

## Phase 3: User Story 1 — Drag to create (Priority: P1) 🎯 MVP

**Goal**: Press-drag on the calendar grid to open a prefilled entry form and save a timed entry that renders at the dragged slot.

**Independent Test**: In Calendar view, drag 9:00→11:30 on a day, confirm the form opens with that day/start/duration, save, and confirm the entry renders at 9:00 (2:30 tall) and persists after reload (quickstart scenario 1).

- [X] T009 [US1] Add calendar drag state (`Signal<Option<{day, start_minute, cur_minute}>>`) and pointer handlers (mousedown on a column, mousemove/mouseup on the grid) with a live "ghost" block, in `crates/horae/src/pages/timesheet.rs` + ghost CSS in `crates/horae/assets/css/horae.css` (D6).
- [X] T010 [US1] On drag release, `snap` + `clamp_to_day` (core) and open the entry form prefilled with the day, `start_minute`, and duration; a zero-movement press opens the form with no start (today's click behavior) — `crates/horae/src/pages/timesheet.rs`.
- [X] T011 [US1] Wire the entry form's Save to pass `start_minute` into `create_time_entry` (reuse the shared persist path) so the created block is timed — `crates/horae/src/pages/timesheet.rs`.
- [X] T012 [US1] Integration test in `crates/horae/tests/integration.rs`: creating with a `start_minute` round-trips and lands within the day; snap/clamp applied.

**Checkpoint**: US1 is an independently shippable MVP.

______________________________________________________________________

## Phase 4: User Story 2 — See entries at their real time (Priority: P2)

**Goal**: Every timed entry (including timer-stopped ones) shows at its hour; overlaps stay visible.

**Independent Test**: Give a day two entries with different start times → each renders at its own position; stop a running timer → the entry appears at the started time (quickstart scenarios 2, 3).

- [X] T013 [US2] On `stop_timer` (`crates/horae/src/server_fns/time_entries.rs`), set the finished entry's `start_minute` from `started_at`'s local time-of-day (snapped) per D9.
- [X] T014 [US2] Lay out overlapping timed blocks side by side so none is hidden, in `render_calendar_view` + CSS (`crates/horae/src/pages/timesheet.rs`, `crates/horae/assets/css/horae.css`) (Edge Cases, SC-005).
- [X] T015 [US2] Integration test in `crates/horae/tests/integration.rs`: stopping a timer records a `start_minute` matching the start (SC-006); two timed entries keep independent positions.

______________________________________________________________________

## Phase 5: User Story 3 — Keep untimed entries working (Priority: P3)

**Goal**: Quick-add/Week-grid/pre-feature entries stay untimed and unchanged; the form can set/clear a start time.

**Independent Test**: Add via the Week grid (duration only) → it stacks from the top and totals are unchanged; clear a timed entry's start in the form → it becomes untimed (quickstart scenario 4).

- [X] T016 [US3] Add a start-time field to the entry form (set/change/clear) that maps to `update_time_entry`'s `start_minute`; clearing sends `None` → untimed, duration unchanged — `crates/horae/src/pages/timesheet.rs` (FR-007).
- [X] T017 [US3] Integration test in `crates/horae/tests/integration.rs`: quick-add creates `start_minute = NULL`; day/week totals equal the exact sum of minutes with a mix of timed/untimed (SC-003, SC-004); pre-existing rows unaffected.

______________________________________________________________________

## Phase 6: User Story 4 — Adjust by dragging (Priority: P3)

**Goal**: Resize a timed block by dragging its edge and move it by dragging its body (including to another day); locked entries are protected.

**Independent Test**: Drag a block's bottom edge → duration changes; drag its body to another hour/day → start/date change; both persist; a submitted entry cannot be moved (quickstart scenario 5).

- [X] T018 [US4] Add `reschedule_time_entry(entry_id, spent_date, start_minute, minutes)` server fn (snap + clamp, `open`-state lock check) in `crates/horae/src/server_fns/time_entries.rs` per [contracts/server-fns.md](./contracts/server-fns.md).
- [X] T019 [US4] Calendar resize: drag a timed block's edges to change duration (bottom) or start+duration (top), committing via `reschedule_time_entry` — `crates/horae/src/pages/timesheet.rs` + CSS handles.
- [X] T020 [US4] Calendar move: drag a timed block to a different time or day, committing via `reschedule_time_entry`; suppress drag on locked entries with a message — `crates/horae/src/pages/timesheet.rs`.
- [X] T021 [US4] Integration test in `crates/horae/tests/integration.rs`: reschedule round-trips date/start/duration; a non-`open` entry is rejected (FR-013).

______________________________________________________________________

## Phase 7: Polish & Cross-Cutting

- [ ] T022 [P] Map `start_minute` → `started_time`/`ended_time` (nullable) in the Harvest v2 read shape per [contracts/harvest-api.md](./contracts/harvest-api.md) — `crates/horae/src/harvest/`.
- [ ] T023 [P] Regenerate `.sqlx/` after all query changes; run `cargo clippy -p horae --features server` and `nix fmt` until clean (Constitution V).
- [ ] T024 [P] Walk quickstart.md scenarios 1–8 against `dx serve`; confirm totals invariant (SC-003) and no regression to existing entries.

______________________________________________________________________

## Dependencies & Order

- **Phase 1 → Phase 2 → Phases 3–6 → Phase 7.**
- Setup (T001→T002) blocks everything (schema + cache).
- Foundational (T003–T008) blocks all user stories. Within it: T003/T004 (core) and T005 (DTO) are parallel; T006/T007 need T005+T003; T008 needs T005.
- US1 (T009–T012) is the MVP and depends only on Foundational.
- US2, US3, US4 each depend on Foundational and are otherwise independent of one another (can be built/tested in any order after US1).
- Polish (T022–T024) runs after the stories it touches; T022 needs T005, T023/T024 run last.

## Parallel Opportunities

- **Foundational**: T003, T004, T005 in parallel (different files).
- **Across stories** (after Foundational): US2, US3, US4 can be developed in parallel by different people — they touch overlapping files (`timesheet.rs`, `time_entries.rs`), so serialize edits to those files or land US1 first.
- **Polish**: T022 (harvest) is independent of T023/T024.

## Implementation Strategy

- **MVP = Phase 1 + Phase 2 + Phase 3 (US1)**: schema, foundation, and drag-create — a shippable, demoable slice.
- Then layer US2 (timer/positioning polish), US3 (explicit untimed form control + compat tests), and US4 (resize/move) incrementally, each behind its own checkpoint and tests.
