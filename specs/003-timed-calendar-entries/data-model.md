# Phase 1 Data Model: Timed Calendar Entries

## Schema change

One nullable column on the existing `time_entries` table. No new tables; PKs remain UUID v7; `org_id` already present.

### `time_entries` (added)

| Column | Type | Null | Notes |
|--------|------|------|-------|
| `start_minute` | `integer` | YES | Minutes since local midnight, 0–1439. `NULL` = untimed (duration-only). |

**Constraints** (in migration `0005_time_entry_start_minute.sql`):

```sql
ALTER TABLE time_entries
  ADD COLUMN start_minute integer,
  ADD CONSTRAINT time_entries_start_minute_range
    CHECK (start_minute IS NULL OR (start_minute >= 0 AND start_minute <= 1439)),
  ADD CONSTRAINT time_entries_within_day
    CHECK (start_minute IS NULL OR start_minute + minutes <= 1440);
```

- Additive and backfill-free: existing rows get `NULL` (untimed) and behave exactly as before.
- `time_entries_within_day` ties the start to the existing `minutes` column so an entry can never cross midnight (D4).
- Regenerate `.sqlx/` after the migration (`cargo sqlx prepare --workspace -- --features server --all-targets`).

## Shared DTO (`models/time_entry.rs`)

`TimeEntry` gains:

```
start_minute: Option<i32>   // 0..=1439, None = untimed
```

Kept as `Option<i32>` (not a newtype) to match the existing DTO style and sqlx mapping; validity is enforced by the DB CHECK and the core helpers below. Compiles for both server and web targets (used by the calendar and the entry form).

## Core domain (`horae-core`, new `time_of_day.rs`) — pure, unit-tested

Correctness-critical, exact, no I/O (Constitution II):

| Function | Signature (intent) | Rules |
|----------|--------------------|-------|
| `parse` | `&str -> Option<u16>` | "9:00", "9:00am", "13:30" → minutes since midnight; reject out-of-range/garbage. |
| `format` | `u16 -> String` | minutes → "9:00" (and/or 12h form for the Harvest mapping). |
| `snap` | `(i32, step: u16) -> i32` | round to nearest `step` (15). Reused by create/resize/move. |
| `clamp_to_day` | `(start: u16, minutes: u32) -> u32` | shrink `minutes` so `start + minutes <= 1440`; the no-cross-midnight rule (D4). |
| `min_duration` | `u16` const | one snap step (15) — a sub-step drag becomes this, never 0 (Edge Cases). |

These are the only new "rules"; pixel↔minute placement stays in the UI layer.

## Validation rules (from FR)

- **FR-001 / D1**: `start_minute ∈ [0,1439] ∪ {NULL}` — enforced by CHECK + `parse`.
- **FR-012 / D4**: `start_minute + minutes ≤ 1440` — enforced by CHECK + `clamp_to_day`.
- **FR-008 / D3**: all created/edited starts and durations are multiples of the snap step — enforced by `snap` at every write path.
- **FR-011**: `start_minute` never participates in any total, rounding, or money calculation — it is scheduling metadata; `minutes`/`rounded_minutes` remain the sole length inputs.
- **FR-010**: `NULL` is always valid and is the default for every existing and quick-add entry.

## State & lifecycle

- No new state machine. `start_minute` is orthogonal to `entry_state` (`open`/`submitted`/`approved`/`invoiced`).
- **FR-013**: move/resize (which writes `start_minute`/`spent_date`/`minutes`) is rejected when the entry is not `open`, reusing the existing lock check on edits.
- **D9**: stopping a running timer sets `start_minute` from `started_at`'s local time-of-day as part of the existing stop transition.

## Entity relationships

Unchanged. `time_entries` still belongs to `org`, `user`, `project`, `task`. `start_minute` is a scalar attribute of the entry; the calendar's "timed vs untimed" split is derived (`start_minute.is_some()`), not a stored flag.
