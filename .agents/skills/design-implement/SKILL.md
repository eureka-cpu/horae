---
name: design-implement
description: Use when implementing or aligning a Horae UI page/screen/component to match the design/ handoff bundle (the .dc.html prototypes from claude.ai/design), or when asked to "build the design", "match the mockup", or turn a design surface into a reviewed PR.
---

# design-implement

## Overview

Turn one surface from the `design/` handoff bundle into **one reviewed PR** that
matches it, using the repo's token + utility CSS system and following `AGENTS.md`.

**Core principle:** the `.dc.html` prototypes are a *visual spec*, not code to
paste. Recreate their visual output in Dioxus + the existing CSS framework; never
copy the prototype's inline styles, hex, or internal structure.

## When to use

- A `design/project/**/*.dc.html` surface needs to exist for real in the app.
- "Match the mockup / build the design / rebuild <page> to the design."
- Keeping the Design System or Components page in sync.

Prior art to imitate: #54 (sidebar), #58 (timesheet). Timesheet
(`src/pages/timesheet.rs`) is the canonical example of translating a `.dc.html`
into this codebase.

## Pipeline

1. **Read the source of truth in full.** Read the target
   `design/project/app/Horae <Surface>.dc.html` top to bottom and follow its
   imports (`support.js`, the Design System page, the Components page). Do **not**
   render or screenshot unless the user asks — every dimension/color/layout rule is
   in the source (per `design/README.md`).
2. **Ignore the prototype's shell.** The rail/sidebar in the mockup is
   reference-only; `src/components/sidebar.rs` + the app shell already wrap every
   page. Implement only the main panel.
3. **Existing page or new screen?**
   - *Existing* (`src/pages/<surface>.rs` present): align it in place — minimal
     scoped diff, preserve working data/handlers.
   - *New screen*: scaffold it — add `src/pages/<name>.rs`, register `pub mod` in
     `src/pages.rs`, add a `#[route(...)]` variant in `src/route.rs`, and a nav
     entry in `src/components/sidebar.rs` / `nav.rs`.
4. **Style with the framework, not inline styles** (see next section).
5. **Stay data-honest.** Don't fabricate UI for data the backend can't produce.
   Compute what the schema supports; omit or defer the rest and **list the
   deviation in the PR body**. Backend/schema changes are a separate concern from a
   visual PR — keep scope tight; if a surface needs new data, confirm with the user.
6. **Verify** (see Verification).
7. **Open one scoped PR.** Human-style title/body, **no AI attribution / no
   `Co-Authored-By`** (`AGENTS.md` → Repository Hygiene). Stop at "PR opened for
   review"; never self-merge.

## Styling contract (most common failure)

Style with the **Tailwind-esque utility framework** — this is required, not
optional:

- **Compose utility classes in markup** for layout/spacing/type: `flex`,
  `items-center`, `justify-between`, `gap-4`, `p-4`, `text-sm`, `font-semibold`,
  `bg-secondary`, `rounded-lg`, responsive `md:`/`lg:` variants. The numeric scale
  matches Tailwind (`p-4` = `--space-4` = 1rem).
- **Reach for a semantic component class** (`.btn`, `.card`, `.badge`,
  `.nav-item`) or an existing `src/components/*` before writing new markup.
- **Map the prototype's literals to tokens**: every hex/px in the `.dc.html`
  becomes a `--color-*` / `--space-*` / `--radius-*` / `--font-size-*` from
  `assets/css/horae.css` (e.g. `#1A1813`→`--color-bg-secondary`,
  `#322E26`→`--color-border`, `#4FB79A`→`--color-primary`). Cross-check the
  DESIGN.md token table.
- **No inline `style=`, no hardcoded colors** (see #63). If a needed token/utility
  is missing, add the token to `horae.css` (or the scale in `crates/horae/build.rs`
  — which regenerates `horae-utils.css`) rather than hardcoding at the call site.
- Page-specific structural CSS goes in `horae.css` with a prefixed class family
  (e.g. `ts-*` for timesheet); reuse tokens inside it.

## Verification (before claiming done)

```sh
SQLX_OFFLINE=true cargo clippy -p horae --features server -- -D warnings
cargo check -p horae --features web --target wasm32-unknown-unknown
nix fmt
```

If any `sqlx::query*!` macro or migration changed, regenerate against a live DB:
`cargo sqlx prepare --workspace -- --features server --all-targets` then
`git add .sqlx/` (the `--features server` flag is mandatory or the cache is wiped).
Live visual check via `dx serve` / the gallery (`src/pages/gallery.rs`) is optional
— use it when layout is uncertain, not as the default gate.

## Common mistakes

| Mistake | Fix |
|---------|-----|
| Working on `master` | Always an isolated `.worktrees/` workspace + branch. |
| Inline `style=` / hardcoded hex | Utility classes + tokens; add a token if missing. |
| Copying the prototype's sidebar/structure | Match visuals only; app shell owns the chrome. |
| Fabricating numbers for unbacked fields | Compute what exists; flag deferrals in the PR. |
| `Co-Authored-By` / AI mention in commit/PR | Human-style messages only. |
| Bundling backend/data rework into a visual PR | Keep scope tight; split or confirm first. |

## Batch tier (opt-in)

To clear several surfaces at once, run the same pipeline across a backlog with the
`Workflow` tool — one worktree/branch/PR per surface, in parallel. See
`batch-workflow.js` in this directory. Requires explicit user opt-in per run
(spawns many agents); the default remains one surface → one reviewed PR.
