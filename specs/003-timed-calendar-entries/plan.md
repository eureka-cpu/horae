# Implementation Plan: Timed Calendar Entries

**Branch**: `003-timed-calendar-entries` | **Date**: 2026-08-04 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/003-timed-calendar-entries/spec.md`

## Summary

Add an optional **start time** to time entries and make the Timesheet Calendar place timed entries at their real hour (untimed entries keep stacking from the top of the day). Users can **drag on the calendar grid to create** an entry for a slot (MVP), and later **resize/move** existing timed entries by dragging — matching Harvest, whose API models this as a nullable `started_time`/`ended_time` (type *time*) alongside a duration.

Technical approach: store the start time as an exact integer **minutes-since-midnight** column (nullable) on `time_entries`; keep `minutes` (duration) as the single source of truth for length so all totals and billing are unchanged. Pure time-of-day helpers (parse/format/snap/clamp) live in `horae-core`. Mutations extend the existing `#[server]` functions; the calendar view gains start-time positioning and pointer-drag interactions.

## Technical Context

**Language/Version**: Rust (edition 2024)

**Primary Dependencies**: Dioxus 0.7 (fullstack + router, SSR + WASM), Axum, sqlx (compile-time-checked macros), chrono, uuid (v7)

**Storage**: PostgreSQL 15+; ordered migrations under `crates/horae/migrations/`; `.sqlx/` offline cache committed

**Testing**: `cargo test -p horae-core` (pure unit); `#[sqlx::test]` + `#[serial]` integration in `crates/horae/tests/`

**Target Platform**: Linux server (Axum) + WebAssembly SPA (Dioxus web)

**Project Type**: Web application (single feature-gated crate `horae`, two targets) + pure `horae-core` domain crate

**Performance Goals**: Calendar drag/preview interaction feels immediate (~60 fps, no server round-trip during drag); data volume is a single user's week (≤ a few hundred entries)

**Constraints**: Durations and the start time are exact **integers** (no floats); a start time is a whole-minute time of day (0–1439); an entry never crosses midnight (`start + duration ≤ 1440`)

**Scale/Scope**: Single organization; existing Timesheet Day/Week/Calendar surface; additive change touching one migration, `horae-core`, a few `#[server]` functions, the Harvest-compatible read mapping, and the calendar renderer.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Assessment |
|-----------|------------|
| **I. Exactness** | ✅ Start time stored as integer minutes-since-midnight; duration stays integer minutes; end is derived (`start + minutes`), never stored, so no new float and no total can drift. A start time is display/scheduling metadata only — it changes no monetary or duration total. |
| **II. Domain Purity** | ✅ Correctness-critical time math — time-of-day parse/format, snapping to the increment, and the `start + duration ≤ 1440` (no-cross-midnight) rule — lives in `horae-core` and is unit-tested there. Pixel↔minute placement math stays in the UI (not correctness-critical). |
| **III. Single Datastore** | ✅ One nullable column added to `time_entries` via an ordered Postgres migration; PKs remain UUID v7; `org_id` already present. `.sqlx/` cache regenerated. |
| **IV. Mutations Through Server Functions** | ✅ All writes go through existing/extended `#[server]` functions (create, update, reschedule, stop-timer). No new client-side fetch; the calendar drag ends by calling a server function. The Harvest v2 read API gains only a mapped field, not a mutation path. |
| **V. Reproducible Builds & Formatting Gate** | ✅ `nix fmt` / `nix flake check` must be green; migration + `.sqlx` prepare committed; no toolchain assumptions. |

**Result**: PASS — no violations. Complexity Tracking not required.

## Project Structure

### Documentation (this feature)

```text
specs/003-timed-calendar-entries/
├── plan.md              # This file
├── research.md          # Phase 0 — decisions & rationale
├── data-model.md        # Phase 1 — schema + core types + rules
├── contracts/
│   ├── server-fns.md    # Dioxus #[server] mutation/read signatures
│   └── harvest-api.md   # Harvest v2 time_entry field mapping
├── quickstart.md        # Phase 1 — end-to-end validation guide
└── tasks.md             # Phase 2 — created by /speckit-tasks
```

### Source Code (repository root)

```text
crates/core/src/
├── duration.rs          # (exists) minute parsing/formatting
└── time_of_day.rs       # NEW: minutes-since-midnight parse/format, snap, day-clamp helpers (pure)

crates/horae/migrations/
└── 0005_time_entry_start_minute.sql   # NEW: add nullable start_minute + CHECK

crates/horae/src/
├── models/time_entry.rs         # add start_minute: Option<i32> to the shared DTO
├── server_fns/time_entries.rs   # extend create/update; add reschedule; set start on stop_timer
├── harvest/…                    # map start_minute → started_time/ended_time in the v2 read shape
└── pages/timesheet.rs           # calendar: position timed by start; untimed stack; drag-create (P1); resize/move (P2)

crates/horae/assets/css/horae.css   # calendar ghost/drag + timed-block styles
crates/horae/tests/integration.rs   # start_minute round-trip, reschedule, stop-timer start
```

**Structure Decision**: Reuse the existing two-crate layout (`horae-core` pure domain + feature-gated `horae` app). The only new files are one migration, one pure `horae-core` module, and design docs; everything else extends existing modules. This keeps the change additive and inside established seams (Constitution II/III/IV).

## Complexity Tracking

No constitutional violations — section intentionally empty.
