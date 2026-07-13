# Implementation Plan: Additional Plugin Events

**Branch**: `002-additional-plugin-events` | **Date**: 2026-07-13 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-additional-plugin-events/spec.md`

## Summary

Extend the plugin event catalog delivered in feature `001` (User Story 5) with additional
business events. Most are **direct**: emit a new `AppEvent` variant immediately after an
existing `#[server]` mutation commits, reusing the current envelope and non-blocking,
failure-isolated `dispatch`. A few are **derived**: budget threshold / over-budget events
computed (in `horae-core`, integer-only) when consumption changes, and `timer_running_too_long`
produced by a periodic background task. The change is additive — the manifest, envelope, host
functions, and sandbox guarantees are untouched.

## Technical Context

**Language/Version**: Rust (edition 2024); shared event types compile for both the `server`
target and the WASM `web` target.

**Primary Dependencies**: the existing plugin subsystem (`crates/horae/src/plugin/*` —
`AppEvent`, `PluginRegistry`, `dispatch`), `serde`/`serde_json` (JSON payloads), `chrono`,
`uuid`, `sqlx` (server-only reads/writes), `tokio` (the periodic scheduler). No new external
crates anticipated.

**Storage**: PostgreSQL. Reads for budget consumption and long-running timers; small config
additions for thresholds/limit and dedupe bookkeeping (see [data-model.md](./data-model.md)).

**Testing**: `horae-core` unit tests for the integer budget-crossing math; `#[sqlx::test]`
integration tests that a mutation triggers exactly one event (and none on no-ops); plugin
fixture tests (echo / fail plugins) reused from feature `001`.

**Target Platform**: Linux server (`server` feature) plus the WASM client for shared types.

**Project Type**: Web application (Dioxus fullstack: Axum + Tokio server, WASM SPA).

**Performance Goals**: dispatch stays non-blocking and time-bounded per feature `001`'s SC-006;
the long-timer scheduler polls at a coarse interval (default every 60s) and does one indexed
query per tick.

**Constraints**: Exactness (integer minutes / integer cents, no floats) for all budget math;
events fire only after the triggering mutation commits; a slow/failing plugin never affects the
triggering action; no event on a no-op write.

**Scale/Scope**: Same single-org deployment as the base app; the catalog grows by ~24 direct
event names plus 3 derived events.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Assessment |
|---|---|
| I. Exactness (NON-NEGOTIABLE) | **Pass** — budget threshold/over-budget comparisons use integer minutes and integer cents only; the crossing test lives in `horae-core` as a pure integer function. No floating point. |
| II. Domain Purity | **Pass** — the "which threshold band did consumption cross" decision is a pure function in `horae-core`; the `horae` crate only serializes payloads and dispatches. `horae-core` gains no I/O deps. |
| III. Single Datastore | **Pass** — PostgreSQL only; any new config/bookkeeping columns carry `org_id` context and use existing UUID v7 keys; changes ship as ordered migrations. |
| IV. Mutations Through Server Functions | **Pass** — direct events are dispatched inside existing `#[server]` functions after their writes; no new mutation path. The long-timer scheduler is a server-side **read-only** poller that dispatches events; it performs no data mutation outside a `#[server]` boundary except an idempotent "notified" bookkeeping update (justified below). |
| V. Reproducible Builds & Formatting Gate | **Pass** — no toolchain change; `nix fmt -- --ci` and `nix flake check` remain the gate; new SQL keeps the `.sqlx` offline cache in sync. |

Also honors the constitution's plugin clause: plugins remain sandboxed and read-only — this
feature only *sends* them more events; it grants no new datastore access.

**Result: PASS — no violations.** One item to keep honest during implementation is noted in
Complexity Tracking (the scheduler's bookkeeping write).

## Project Structure

### Documentation (this feature)

```text
specs/002-additional-plugin-events/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── plugin-events-addendum.md   # Phase 1 output — the new event catalog
└── tasks.md             # Phase 2 output (/speckit-tasks — not created here)
```

### Source Code (repository root)

```text
crates/core/src/
└── budget.rs            # NEW — pure integer budget-consumption / threshold-crossing logic

crates/horae/src/
├── plugin/
│   └── event.rs         # EXTEND — new AppEvent variants + payload structs + hook_name arms
├── server_fns.rs        # EXTEND — dispatch calls after existing mutations; no-op suppression
├── auth/                # EXTEND — dispatch user_logged_out on logout
├── scheduler.rs         # NEW (server-only) — periodic long-running-timer check
├── state.rs             # EXTEND — hold config / spawn the scheduler at startup
└── main.rs              # EXTEND — start the scheduler on `serve`

crates/horae/migrations/
└── NNNN_plugin_event_support.sql   # NEW — org thresholds/limit + dedupe bookkeeping columns

crates/horae/tests/
└── integration.rs       # EXTEND — event-emission + no-op-suppression tests
```

**Structure Decision**: Reuse the existing fullstack layout. Correctness-critical budget math
goes in `horae-core` (`budget.rs`); event definitions extend the existing `plugin/event.rs`;
dispatch lives in the existing `#[server]` functions; the only net-new server component is a
small `scheduler.rs` background task for the one time-based event.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| Periodic scheduler + a "notified" bookkeeping write outside a `#[server]` fn (touches Principle IV's spirit) | `timer_running_too_long` is inherently time-based, not triggered by any user mutation; it must be detected by polling, and the write is only an idempotent dedupe marker so the event fires once per overrun | A purely event-driven approach cannot detect "nothing happened for N hours"; firing on every poll tick (no marker) would spam plugins and violate SC-001's "exactly one event" |
