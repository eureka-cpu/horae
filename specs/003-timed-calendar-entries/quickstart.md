# Quickstart: Validate Timed Calendar Entries

Prove the feature end-to-end. See [data-model.md](./data-model.md), [contracts/server-fns.md](./contracts/server-fns.md), and [contracts/harvest-api.md](./contracts/harvest-api.md) for details; this is a run/validate guide only.

## Prerequisites

- Nix dev shell (`nix develop`) with a running PostgreSQL (`DATABASE_URL=postgres://localhost/horae`).
- Migration `0005_time_entry_start_minute.sql` applied and `.sqlx/` regenerated.

## Setup & run

```sh
# from crates/horae
… -- migrate run          # applies 0005 (adds start_minute)
… -- seed                 # sample data
DEV_LOGIN=1 DATABASE_URL=… dx serve   # http://localhost:8080
```

Sign in as Admin, open `/timesheet/calendar/<today's monday>`.

## Automated checks (must pass)

```sh
cargo test -p horae-core time_of_day        # parse/format/snap/clamp_to_day rules (D3, D4)
DATABASE_URL=… cargo test -p horae --features server start_minute   # round-trip, reschedule, stop-timer start
cargo clippy -p horae --features server
nix fmt -- --ci
```

## Manual validation scenarios

Each maps to a spec acceptance scenario / success criterion.

1. **Drag-create (Story 1 / SC-001, SC-002)**: In Calendar view, press at 9:00 in a day column, drag to 11:30, release → the entry form opens prefilled with that day, start 9:00, duration 2:30. Pick project/task, save → a block renders at 9:00, 2½h tall. Reload the URL → still there. Drag upward (release above start) → same slot (direction-agnostic).
2. **Positioning (Story 2 / SC-002)**: An entry with start 14:00 for 1:00 renders beginning at the 14:00 gridline, one hour tall.
3. **Timer start (Story 2 / SC-006)**: Start a timer, wait, stop it → the finished entry appears at the clock time it was started (to the snap step).
4. **Untimed unchanged (Story 3 / SC-004)**: Add an entry via the Week grid (duration only) → in Calendar it stacks from the top of its day; day/week totals equal the exact sum (SC-003). Clear a timed entry's start in the form → it moves to the untimed stack, duration unchanged.
5. **Resize & move (Story 4)**: Drag a timed block's bottom edge down 1h → duration +1:00, same start. Drag its body to 13:00 → start 13:00, same duration. Drag it into another day's column → moves days, keeps start+duration. All persist after reload.
6. **Edge cases**: Drag past end-of-day → clamps to 24:00 (no cross-midnight). A plain click (no drag) → opens the entry form with no start time (today's behavior). Try to move/resize a submitted entry → blocked with a "locked" message.
7. **Totals invariant (SC-003)**: On a mixed day (timed + untimed), the day total and week total equal the exact sum of every entry's minutes — unchanged by any start time.
8. **Harvest read mapping**: Fetch a timed entry via `/harvest/v2/*` → `started_time`/`ended_time` are populated (`H:MMam/pm`); an untimed entry → both `null`.

## Expected outcome

All automated checks green; scenarios 1–8 behave as described; existing (pre-feature) entries display, edit, submit, and total exactly as before.
