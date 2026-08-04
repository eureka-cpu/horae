# Contract: Harvest v2 Read Mapping

The read-only Harvest-compatible API (`/harvest/v2/*`) exposes time entries. This feature maps the new start time onto Harvest's documented fields. **Read-only — no mutation path (Constitution IV).**

## Field mapping (per time entry)

| Harvest field | Type | Source in Horae |
|---------------|------|-----------------|
| `hours` | decimal | `minutes / 60` (unchanged) |
| `started_time` | *time* or `null` | `time_of_day::format_12h(start_minute)` when `start_minute` is set, else `null` |
| `ended_time` | *time* or `null` | `format_12h(start_minute + minutes)` when `start_minute` is set, else `null` |
| `timer_started_at` | datetime or `null` | running-timer `started_at` (unchanged) |
| `is_running` | boolean | unchanged |

Notes:

- Matches Harvest's real object: `started_time`/`ended_time` are nullable *time* fields present only when the entry has a start time (verified against Harvest API v2 docs).
- `ended_time` is **derived** (`start + duration`), consistent with D2 (only start is stored).
- When `start_minute` is `null` (duration-only / untimed), both time fields serialize as `null`, exactly like Harvest duration-only entries.
- No breaking change: existing consumers that ignore the time fields keep working; consumers that read them now get real values when present.

## Non-goals

- No write endpoint is added to the Harvest API.
- 12-hour vs 24-hour formatting for `started_time` follows Harvest's `H:MMam/pm` convention; the exact formatter lives in `horae-core::time_of_day`.
