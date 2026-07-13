# Feature Specification: Additional Plugin Events

**Feature Branch**: `002-additional-plugin-events`

**Created**: 2026-07-13

**Status**: Draft

**Input**: User description: "Additional plugin events for the Horae plugin system — an extension of User Story 5 (plugins) and FR-019. Beyond the initial five events, add a catalog of further business events plugins can subscribe to, plus derived/computed events, all reusing the same envelope and non-blocking, failure-isolated dispatch."

This specification is an **addendum** to the plugin system delivered in feature `001` (User Story 5, FR-018–FR-022). It only *grows the set of events* plugins may subscribe to; the plugin manifest, the event envelope, the host functions, the dashboard-widget contract, and the sandbox/failure-isolation guarantees are unchanged.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - React to the full time & invoicing lifecycle (Priority: P1)

A plugin author wants their plugin to stay in sync with everything that happens to time entries and invoices — not just when an entry is created/stopped or an invoice is created/sent. They declare the relevant hooks in `plugin.toml` and their plugin is invoked whenever an entry is edited or deleted, a timesheet is submitted/approved/rejected, or an invoice is paid or voided.

**Why this priority**: These are the money-and-time state changes an integration most needs to mirror (accounting sync, notifications, audit). Without them, a plugin sees only part of each entity's life and can drift out of sync. This is the smallest set that makes plugin-based integrations trustworthy, so it is the MVP of this addendum.

**Independent Test**: Install a plugin subscribed to `invoice_paid` and `timesheet_submitted`; mark an invoice paid and submit a timesheet; confirm the plugin receives one well-formed event per action and nothing else.

**Acceptance Scenarios**:

1. **Given** a plugin subscribed to `time_entry_updated`, **When** a user edits an existing entry, **Then** the plugin receives one `time_entry_updated` event carrying the updated entry.
1. **Given** a plugin subscribed to `time_entry_deleted`, **When** a user deletes an entry, **Then** the plugin receives one `time_entry_deleted` event identifying the removed entry.
1. **Given** a plugin subscribed to `timesheet_submitted`, `submission_approved`, and `submission_rejected`, **When** a timesheet moves through submit → approve (or reject), **Then** the plugin receives one event per transition, in order.
1. **Given** a plugin subscribed to `invoice_paid`, **When** an invoice's status changes to paid, **Then** the plugin receives one `invoice_paid` event; **When** it changes to void, **Then** it receives `invoice_voided`.

______________________________________________________________________

### User Story 2 - React to administrative & catalog changes (Priority: P2)

A plugin author wants to react to changes in the organization's people and catalog: users being created, having their role changed, being deactivated or logging out; clients, projects, and tasks being created, edited, deactivated or reactivated; users being assigned to or removed from projects; and organization branding being updated. This supports provisioning, access auditing, and syncing directory/catalog data to external systems.

**Why this priority**: Valuable for governance and integration but secondary to the core money-and-time flows. An organization can operate without these hooks; they add reach for admin-oriented plugins.

**Independent Test**: Install a plugin subscribed to `user_role_changed` and `project_deactivated`; change a user's role and deactivate a project; confirm the role-change event carries both the previous and new role, and that reactivating the project later emits `project_reactivated`.

**Acceptance Scenarios**:

1. **Given** a plugin subscribed to `user_created` and `user_role_changed`, **When** an admin creates a user and later changes their role, **Then** the plugin receives `user_created` and then `user_role_changed` including the previous role.
1. **Given** a plugin subscribed to `client_deactivated`/`client_reactivated`, **When** a manager toggles a client's active flag, **Then** exactly one matching event fires per real change of state.
1. **Given** a plugin subscribed to `user_assigned_to_project` and `assignment_removed`, **When** a user is assigned to and later removed from a project, **Then** the plugin receives one event for each.

______________________________________________________________________

### User Story 3 - Proactive budget & long-timer alerts (Priority: P3)

A plugin author wants to be warned *before* problems compound: when a project's tracked time or amount crosses a configured share of its budget, when it goes over budget, and when a timer has been left running far too long. These are derived from the system's state rather than emitted verbatim after a single action.

**Why this priority**: High operational value (avoid budget overruns, catch forgotten timers) but the most involved to produce — it requires computing consumption against budget and, for long-running timers, a periodic check rather than a reaction to a single write. Deferred to last so the simpler direct events land first.

**Independent Test**: Configure a project with a budget and an 80% threshold; log time until consumption crosses 80%; confirm a single `project_budget_threshold_reached` event fires as the threshold is crossed, and a later crossing of 100% fires `project_over_budget`.

**Acceptance Scenarios**:

1. **Given** a plugin subscribed to `project_budget_threshold_reached` and a project at 79% of budget, **When** an action pushes consumption to 82%, **Then** exactly one threshold event fires carrying the project, the threshold percentage, and consumed-vs-budget figures.
1. **Given** consumption already past a threshold, **When** further time is logged without crossing a new threshold, **Then** no duplicate threshold event fires.
1. **Given** a plugin subscribed to `timer_running_too_long` and a timer running beyond the configured limit, **When** the periodic check runs, **Then** the plugin is notified once for that timer.

______________________________________________________________________

### Edge Cases

- **No-op writes**: An update that does not change any field, or a set-active call with the same value, MUST NOT emit an event (events represent real state changes).
- **Failing/slow plugin**: A plugin subscribed to a new event that errors or hangs MUST NOT affect or delay the action that triggered it, exactly as for the existing events.
- **Action rolls back**: If the underlying mutation fails and is not committed, no event fires (dispatch happens only after commit).
- **Threshold already crossed at creation**: If a project is already over a threshold when consumption is first computed, at most one crossing event fires; it does not re-fire on every subsequent entry.
- **Void restores entries**: `invoice_voided` fires once for the invoice; any resulting change to the freed time entries follows the existing entry-event rules and does not double-notify.
- **Unknown hook name**: A manifest declaring a hook not in the (now larger) catalog is still rejected by existing manifest validation.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST notify subscribed plugins when a time entry is edited (`time_entry_updated`) or deleted (`time_entry_deleted`).
- **FR-002**: The system MUST notify subscribed plugins when a timesheet is submitted (`timesheet_submitted`), approved (`submission_approved`), or rejected (`submission_rejected`).
- **FR-003**: The system MUST notify subscribed plugins when an invoice becomes paid (`invoice_paid`) or void (`invoice_voided`).
- **FR-004**: The system MUST notify subscribed plugins when a client, project, or task is created or updated, and when its active state actually flips (`*_deactivated` / `*_reactivated`).
- **FR-005**: The system MUST notify subscribed plugins on user lifecycle changes: `user_created`, `user_role_changed` (carrying the previous role), `user_deactivated`, and `user_logged_out`.
- **FR-006**: The system MUST notify subscribed plugins when a user is assigned to a project (`user_assigned_to_project`) or an assignment is removed (`assignment_removed`).
- **FR-007**: The system MUST notify subscribed plugins when organization branding is updated (`org_branding_updated`).
- **FR-008**: The system MUST notify subscribed plugins when a project's consumption crosses a configured share of its budget (`project_budget_threshold_reached`) and when it exceeds its budget (`project_over_budget`), including consumed-vs-budget figures; each threshold crossing fires at most once.
- **FR-009**: The system MUST notify subscribed plugins when a timer has been running longer than a configured limit (`timer_running_too_long`), evaluated on a periodic schedule rather than in reaction to a single write.
- **FR-010**: Every new event MUST use the existing envelope (`event`, `occurred_at`, `org_id`, and one payload object) and be identified by a hook name a plugin declares in `plugin.toml`.
- **FR-011**: New-event dispatch MUST remain non-blocking and failure-isolated: events fire only after the triggering mutation is committed, and a slow or failing plugin MUST NOT affect the triggering action (consistent with the existing dispatch guarantees).
- **FR-012**: The system MUST NOT emit an event when the underlying state did not change (no-op updates and unchanged active flags produce no event).
- **FR-013**: Subscription MUST continue to work through the existing `plugin.toml` `hooks` list; this feature only enlarges the set of valid hook names, and existing manifest validation still rejects unknown names.
- **FR-014**: The change MUST be additive: the five existing events and the plugin manifest, envelope, host functions, dashboard-widget contract, and sandbox guarantees remain unchanged (this is an addendum, not a replacement).
- **FR-015**: Budget thresholds (which percentage(s) trigger `project_budget_threshold_reached`) and the long-timer limit (`timer_running_too_long`) MUST be configurable, with sensible defaults, rather than hard-coded per plugin.

### Key Entities *(include if feature involves data)*

- **Event**: A past-tense business fact delivered to a plugin. Common attributes: hook name, time it occurred, owning organization, and a single typed payload.
- **Submission payload**: Identifies a timesheet submission — who, which week, its status, and its total tracked minutes.
- **Client / Project / Task payload**: Identifies the catalog entity and the attributes a plugin would act on (name, currency/type/budget mode, active flag).
- **Assignment payload**: Links a user to a project with a role.
- **Budget-threshold payload**: The project, the threshold percentage crossed, and consumed-vs-budget figures (in minutes and/or minor currency units).
- **Previous role**: The role a user held before a `user_role_changed` event, delivered alongside the new role.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For each new event, 100% of qualifying actions deliver exactly one well-formed event (correct hook name and complete payload) to every subscribed plugin.
- **SC-002**: A no-op write (unchanged update or unchanged active flag) produces zero events.
- **SC-003**: A plugin subscribed to a new event that errors or hangs causes no failure of, and no measurable added latency to, the action that triggered it — matching the existing dispatch time bound.
- **SC-004**: Each budget threshold crossing produces exactly one event — no missed crossings and no duplicates when consumption keeps rising without crossing a new threshold.
- **SC-005**: The five pre-existing events retain identical behavior and payloads (100% backward compatible; existing plugins need no changes).
- **SC-006**: A plugin author can begin receiving any new event by adding only its hook name to `plugin.toml` and exporting a matching function — no other configuration.

## Assumptions

- Builds directly on the plugin system from feature `001` / User Story 5 (registry, manifest hooks, and the `dispatch` mechanism); that work is the dependency for this addendum.
- Direct events (User Stories 1–2) each correspond to an existing state-changing action, so they can be emitted immediately after that action commits.
- Budget thresholds default to a single 80% warning plus a 100% over-budget event, configurable per organization (and optionally per project); exact configuration surface is left to planning.
- The long-running-timer limit defaults to 8 hours and is configurable; delivering `timer_running_too_long` depends on a periodic scheduler that does not yet exist in the base system and is therefore the largest piece of net-new work.
- Event payloads mirror fields already exposed by existing events/models and introduce no personal data beyond what plugins already receive.
- "Configurable" thresholds are administrative settings, not per-plugin parameters, so multiple plugins observe consistent threshold events.
