# Design-implementation workflow

Automate the recurring work of turning the `design/` handoff bundle into reviewed
UI PRs, the way #52 (component kit), #54 (sidebar), and #58 (timesheet) were done
by hand.

## Context

`design/` is a [claude.ai/design](https://claude.ai/design) handoff bundle:
HTML/CSS/JS prototypes (`.dc.html`, with an `<x-dc>`/`<sc-if>` templating layer
loaded by `support.js`). The `app/` folder holds full-page references — Approvals,
Projects, Reports, Settings, Timesheet — alongside a **Design System** page and a
**Components** page. The bundle's `design/README.md` is explicit: recreate the
prototypes *pixel-perfectly* in the target stack, matching visual output rather
than copying the prototype's internal structure.

The target stack is already in place: one-file-per-component `#[component]`
functions in `crates/horae/src/components/`, pages in `crates/horae/src/pages/`,
and a token + generated-utility CSS system (`assets/css/horae.css` +
`assets/css/horae-utils.css` from `crates/horae/build.rs`). Most design pages
already have a `src/pages/*.rs` counterpart, so that work is *alignment* — but the
workflow must also handle **new screens** (a design surface with no page yet):
scaffold the page module, route, and nav entry, then implement.

## Goal

A repeatable, low-variance path from "a design surface" to "one reviewed PR that
matches it", that always follows the practices in `AGENTS.md` (token/utility
styling, minimal scoped diffs, build/clippy/fmt verification, human-style commits
with no AI attribution, review-before-merge).

## Two-tier shape

1. **`design-implement` skill (default).** An on-demand playbook invoked per
   surface. One surface → one worktree → one scoped PR. This is the unit of work
   and the only tier most sessions use. It keeps a human review gate on every
   change, matching the repo's culture.

1. **Batch Workflow (opt-in).** A thin `Workflow`-tool script that runs the *same*
   playbook across several backlog surfaces in one invocation, one branch/PR each,
   in parallel worktrees. Used only to clear a backlog; explicit opt-in per run.
   It calls the skill's steps rather than duplicating them (single source of
   truth).

## The pipeline (shared by both tiers)

1. **Read the source of truth in full.** Read the target
   `design/project/app/Horae <Surface>.dc.html` top to bottom and follow its
   imports (`support.js`, the Design System page, the Components page). Do not
   screenshot or render unless the user asks — dimensions/colors/layout are in the
   source (per the bundle README).
1. **Map, don't copy.** Translate the prototype's hardcoded hex/px into the
   codebase idiom: `assets/css/horae.css` tokens (`--color-*`, `--space-*`,
   `--radius-*`, `--font-size-*`) and the Tailwind-style utility classes
   (`flex`, `gap-4`, `p-4`, `text-sm`, `bg-secondary`, …). Reuse a semantic
   component class (`.btn`, `.card`, `.badge`, `.nav-item`) or an existing
   `src/components/*` before adding markup. No inline styles (see #63), no
   hardcoded colors.
1. **Diff intent vs. reality.** Compare the prototype against the current
   `src/pages/<surface>.rs` and the components it uses; enumerate concrete gaps
   (layout, spacing, states, missing pieces) before editing. For a **new screen**,
   scaffold first: add `src/pages/<name>.rs`, register `pub mod` in
   `src/pages.rs`, a `#[route(...)]` variant in `src/route.rs`, and a nav entry in
   `src/components/sidebar.rs` / `nav.rs`.
1. **Implement in a worktree.** Isolated `.worktrees/` workspace; extend the
   component kit when a piece is reused rather than inlining one-offs.
1. **Cleanup & framework-consistency review.** Review the diff against the utility
   framework: delete page CSS that merely re-expresses utilities, de-duplicate new
   classes against existing utilities/tokens/semantic components, remove dead
   classes, and confirm no inline styles or hardcoded hex survived. This phase is
   what keeps the utility layer from being quietly reinvented per page.
1. **Verify before claiming done.** `SQLX_OFFLINE=true cargo clippy -p horae --features server -- -D warnings`, `cargo check -p horae --features web --target wasm32-unknown-unknown`, `nix fmt`. Optionally exercise the surface in
   the component gallery (`src/pages/gallery.rs`).
1. **Open one scoped PR.** Human-style title/body, no AI attribution or
   `Co-Authored-By` lines (`AGENTS.md` → Repository Hygiene). Stop at "PR opened
   for review".

## Styling contract (the part most likely to go wrong)

The prototypes are full of inline `style="…#100F0C…"`. The single most important
rule: **those are a visual spec, not code to paste.** A correct implementation has
no inline hex and no inline `style` for anything the utility/token layer already
expresses. When a needed token or utility is missing, add the token to
`horae.css` (or the scale in `build.rs`) rather than hardcoding at the call site.

## Backlog

Timesheet (#58) and the sidebar (#54) are done. Candidate surfaces: **Approvals,
Projects, Reports, Settings**, plus keeping the Design System / Components pages in
sync. The batch tier exists to take that list in one pass when wanted.

## Non-goals (YAGNI)

- No autonomous merging — every PR is human-reviewed.
- No screenshot/visual-diff harness — the bundle README says read the source; add
  it only if pixel drift becomes a real problem.
- No scheduled/CI trigger for now — on-demand + opt-in batch covers the need.
- No new design tooling; consume the existing bundle as-is.

## Verification of the skill itself

Per `superpowers:writing-skills`, the skill is validated with subagent scenarios:
a baseline run (no skill) to confirm which steps agents skip — expected misses:
reading the whole `.dc.html`, mapping hex→tokens instead of inlining, and omitting
the no-AI-attribution rule — then a with-skill run confirming compliance.
