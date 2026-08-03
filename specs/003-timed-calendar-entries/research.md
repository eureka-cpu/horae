# Phase 0 Research: Timed Calendar Entries

All spec ambiguities were resolved in the `## Clarifications` session; no `NEEDS CLARIFICATION` remain. This file records the technical decisions that shape the design.

## D1 — Start-time representation

**Decision**: Store the start time as an integer **minutes-since-midnight** in a `smallint` column `start_minute` (range 0–1439), nullable. Interpreted in the organization's working day (the same local day as `spent_date`); no time zone is attached.

**Rationale**: Constitution I (Exactness) forbids floats for time and mandates integer minutes; minutes-since-midnight is the exact, index-free, arithmetic-friendly form. It composes directly with the existing integer `minutes` duration (`end = start_minute + minutes`) and with the pixel math in the calendar. It maps trivially to Harvest's API `started_time` (a wall-clock *time*), and back.

**Alternatives considered**:
- `timestamptz` (full instant): rejected — introduces a time zone and an instant where the domain only needs a time of day tied to an existing date; invites float/instant drift and is heavier than needed.
- Postgres `time` type: rejected — reintroduces sub-minute precision and formatting ambiguity; integer minutes is exact and matches the rest of the schema (`minutes`, `round_minutes`).

## D2 — Store start only; derive end

**Decision**: Add only `start_minute`. The end is always `start_minute + minutes`, computed where needed; it is never stored.

**Rationale**: `minutes` (duration) is already the single source of truth for length and drives every total, rounding, and invoice. Storing an end too would create a second source that could disagree. Harvest's API exposes both `started_time` and `ended_time`, but we can derive `ended_time` from start + duration for the read mapping, keeping one writable source (Constitution I).

## D3 — Snap increment

**Decision**: Snap dragged/entered start times and durations to **15 minutes**.

**Rationale**: Common scheduling granularity; keeps the grid tidy and drag math forgiving. Implemented as a pure `snap(minutes, step)` in `horae-core` so it is testable and reused by drag-create, resize, and move. The value is a single constant, easy to tune.

## D4 — No entry crosses midnight

**Decision**: Enforce `start_minute IS NULL OR start_minute + minutes <= 1440` at two layers: a table `CHECK` constraint (integrity) and a pure `clamp_to_day(start, minutes)` in `horae-core` (used by drag/resize so the UI never submits an over-long slot).

**Rationale**: The spec fixes entries to a single date; a DB CHECK makes the invariant impossible to violate, while the core clamp gives a friendly UX (a drag past end-of-day stops at 24:00). Splitting a long session across days is unchanged (separate entries).

## D5 — Untimed entries keep stacking from the top

**Decision**: Entries with `start_minute IS NULL` render exactly as today — stacked from the top of the day, height ∝ duration. Only entries with a `start_minute` are positioned at their hour. A day can show both (untimed stack up top, timed at their times).

**Rationale**: Verified against Harvest live: duration-only entries stack from the top there too. Keeping current behavior for untimed entries makes the change strictly additive (Story 3) and avoids inventing an "all-day lane" that Harvest does not use.

## D6 — Drag interaction (pointer math in the UI)

**Decision**: Implement drag with DOM pointer events on the calendar columns/grid, converting the pointer's page-Y delta to minutes via the fixed pixels-per-hour constant. A `drag` signal holds `{ day, start_minute, cur_minute }` and renders a live "ghost" block during the drag; on release, snap and call the appropriate server function (create → open the prefilled entry form; resize/move → reschedule directly). Pixel↔minute conversion stays in the page (not correctness-critical); minute snapping/clamping calls `horae-core`.

**Rationale**: The drag preview must be instant and local (no server round-trip); only the final commit is a mutation (Constitution IV). Keeping placement math in the UI respects Domain Purity (II) — the exact rules that must never break (snap, day-clamp, parse/format) are the ones pushed to core.

## D7 — Server-function surface

**Decision**: Extend `create_time_entry` and `update_time_entry` to carry `start_minute: Option<i32>`; add a `reschedule_time_entry(id, spent_date, start_minute, minutes)` for calendar move/resize (it can change the date, start, and duration in one authorized call); have `stop_timer` record the timer's real start-of-day minute onto the finished entry.

**Rationale**: One typed, authorized mutation path (IV). Reusing create/update keeps the form flow unchanged for untimed entries; a dedicated reschedule keeps the drag-move (which may cross days) explicit and easy to authorize/validate.

## D8 — Harvest v2 compatibility mapping (read-only)

**Decision**: In the Harvest-compatible read API, map `start_minute` → `started_time` (format `H:MMam/pm`) and `start_minute + minutes` → `ended_time`; both are `null` when `start_minute` is null. Keep `hours` (duration) as today.

**Rationale**: Matches Harvest's documented object (both fields are nullable *time*), so downstream Harvest tooling keeps working and gains the times when present. Read-only surface — no new mutation path (IV).

## D9 — Timer stop sets the start time

**Decision**: When a running timer is stopped, set the resulting entry's `start_minute` to the local time-of-day of the timer's `started_at`.

**Rationale**: A timer inherently has a real start; recording it makes timer entries appear on the calendar at the right hour for free (Story 2 / SC-006) with no extra user action.
