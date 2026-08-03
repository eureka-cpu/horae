# Contract: Server Functions (Dioxus `#[server]`)

All mutations go through session-authenticated, role-checked `#[server]` functions (Constitution IV). Signatures below are intent-level (types abbreviated); errors use the repo's `ServerFnError` with named status codes.

## Changed

### `create_time_entry`

```
create_time_entry(
    project_id: String,
    task_id: String,
    spent_date: String,       // YYYY-MM-DD
    minutes: i32,             // whole minutes, snapped
    notes: Option<String>,
    billable: bool,
    start_minute: Option<i32>,   // NEW — 0..=1439, None = untimed
) -> Result<TimeEntry, ServerFnError>
```

- Server snaps `minutes` and `start_minute` to the 15-min step and applies `clamp_to_day`; rejects `start_minute` outside 0..=1439.
- Backward compatible: existing callers pass `None` (Week grid, plain "+").

### `update_time_entry`

```
update_time_entry(
    entry_id: String,
    minutes: i32,
    notes: Option<String>,
    billable: bool,
    start_minute: Option<i32>,   // NEW — set/change/clear the start time
) -> Result<TimeEntry, ServerFnError>
```

- Clearing the start (`None`) converts the entry to untimed with duration unchanged (FR-007).
- Blocked when the entry is not `open` (FR-013).

### `stop_timer`

```
stop_timer(entry_id: String) -> Result<TimeEntry, ServerFnError>
```

- Unchanged signature; additionally sets `start_minute` from the timer's `started_at` local time-of-day (D9), snapped.

## New

### `reschedule_time_entry` (calendar move / resize — P2)

```
reschedule_time_entry(
    entry_id: String,
    spent_date: String,       // may differ (drag to another day)
    start_minute: i32,        // required here (a timed block is being placed)
    minutes: i32,             // resize changes this; move keeps it
) -> Result<TimeEntry, ServerFnError>
```

- Single authorized call for both gestures: move (changes `spent_date` and/or `start_minute`, same `minutes`) and resize (changes `minutes`, and `start_minute` when the top edge is dragged).
- Server snaps + `clamp_to_day`; rejects when the entry is not `open` (FR-013).

## Unchanged reads

`list_time_entries` returns `TimeEntry` including the new `start_minute` (used by the calendar to position timed blocks). No shape break — it is an added field.

## Validation ownership

Server functions call `horae-core::time_of_day` (`snap`, `clamp_to_day`) so the exact rules are enforced server-side regardless of client, and the DB CHECK is the final backstop.
