# Phase 1 Quickstart & Validation: Time Tracking & Invoicing

A runnable guide to bring the app up and validate the feature end-to-end. It maps setup and checks to the spec's acceptance scenarios and success criteria. Interface details live in `contracts/`; entity details in `data-model.md`.

## Prerequisites

- Nix with flakes enabled.
- A PostgreSQL instance. The easiest source in dev is the flake's VM (`nix run .#qemu-vm`), which forwards Postgres on `localhost:5432`.

## Setup

```sh
nix develop                 # dev shell: rust toolchain, dx, sqlx-cli, postgres, wasm-pack
nix run .#qemu-vm           # (optional) boot a NixOS VM running PostgreSQL

# apply schema, then load demo data
DATABASE_URL=postgres://horae@127.0.0.1:5432/horae cargo run --features server -- migrate run
DATABASE_URL=postgres://horae@127.0.0.1:5432/horae cargo run --features server -- seed

# run the app with the dev-login bypass (no OIDC needed locally)
DEV_LOGIN=1 DATABASE_URL=postgres://horae@127.0.0.1:5432/horae dx serve
```

Open <http://localhost:8080/auth/login> and choose "Sign in as Admin". Command and env-var details: `contracts/cli.md`.

## Validation scenarios

Each maps to a user story in `spec.md`. "Expected" is the pass condition.

### US1 — Track billable time (P1)

1. From the dashboard, start a timer on a seeded project/task → **Expected**: a running entry appears and its elapsed time increments live (AS1).
1. Stop the timer → **Expected**: the entry is saved with start/end and an exact duration; no timer runs (AS2).
1. Add a manual entry (date, duration, project, task, notes) → **Expected**: it appears in the list (AS3).
1. Edit an entry's duration/notes/billable flag → **Expected**: changes persist and day/period totals update (AS4).
1. With a timer running, try to start a second one → **Expected**: prevented / first is stopped, with a clear message (AS5, FR-004).

### US2 — Organize clients, projects, tasks (P2)

1. As a manager, create a client (name + currency), a project (billing method, budget, rate), and a task → **Expected**: the task becomes selectable when logging time (AS1–AS3).
1. Mark a project inactive → **Expected**: it disappears from new-entry pickers but existing entries remain (AS4, FR-011).

### US3 — Invoice tracked time (P3)

1. With billable, un-invoiced time for a client, generate an invoice for a period → **Expected**: a draft invoice whose line items and total reconcile exactly with the selected entries (AS1, FR-012/FR-023).
1. **Expected**: the included entries are now marked invoiced and can't be billed again (AS2, FR-013).
1. Move the invoice draft → sent → paid (or void) → **Expected**: status updates in lists (AS3).
1. Export the invoice → **Expected**: exported amounts match the on-screen invoice exactly (AS4, SC-007).

### US4 — Administer users & access (P4)

1. As an admin, create a member and a manager; sign in as each → **Expected**: the member cannot manage clients/projects/invoices; the manager can (AS1–AS2, FR-001/FR-002).
1. Deactivate a user → **Expected**: they can no longer sign in; their history is intact (AS3).

### US5 — Extend with plugins (P5, planned)

1. Install a sample plugin subscribed to `invoice_sent` under `{dataDir}/plugins/`; restart → **Expected**: the plugin loads and registers (AS1, FR-018).
1. Send an invoice → **Expected**: the plugin's action fires within ~1 s (observable via its effect/log) (AS2, SC-006).
1. Install a plugin that errors or hangs; trigger its event → **Expected**: the core action still completes; the failure is isolated and logged (AS3, FR-021).
1. Install a plugin returning a dashboard widget → **Expected**: its content renders in a designated plugin area (AS4, FR-022).

## Automated checks

```sh
cargo test -p horae-core                         # exact rounding/money/totals/state (FR-023, SC-002)
DATABASE_URL=… cargo test --features server      # integration tests (#[sqlx::test], throwaway DBs)
nix flake check                                  # formatting + full NixOS e2e VM test
nix fmt -- --ci                                  # formatting gate (CI)
```

The e2e check (`nix flake check`) boots the NixOS module, seeds data, and exercises health, dev-login, and the read-only Harvest API (`contracts/harvest-api.md`).

## Success-criteria spot checks

- **SC-001**: starting a timer takes ≤ 3 interactions and < 10 s from the dashboard.
- **SC-002 / SC-007**: on-screen totals, exported reports, and invoices all reconcile to the exact sum of entries.
- **SC-003**: generating a client's period invoice from tracked time takes < 2 minutes.
- **SC-005**: with seed-scaled data (≥ 50 users / 100k entries), list and report views return within 2 s.
- **SC-006**: a subscribed plugin receives its event within 1 s and never blocks the triggering action.
- **SC-008**: an operator can reach a usable state (admin available, first entry recordable) by following this guide, with no external SaaS.
