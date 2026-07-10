# Feature Specification: Time Tracking & Invoicing

**Feature Branch**: `001-time-tracking-invoicing`

**Created**: 2026-07-10

**Status**: Draft

**Input**: User description: "Horae — a self-hostable time-tracking web application (a Harvest/Kimai alternative), distilled from PLAN.md. Users log in and track billable time against clients → projects → tasks via start/stop timers and manual entries; managers manage clients, projects (budgets, billing method, hourly rate), tasks, and users, and generate invoices from tracked time (draft/sent/paid/void); admins manage users and roles; plus a sandboxed plugin system where plugins subscribe to events (time entry created/stopped, invoice created/sent, user logged in) and can contribute dashboard widgets."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Track billable time (Priority: P1)

A team member signs in, starts a timer against the project and task they are working on, and stops it when they switch context. When they forget to run a timer, they add the time by hand. They can see, edit, and correct their own entries, and each entry records whether the work is billable.

**Why this priority**: Capturing accurate time is the core reason the product exists; without it nothing downstream (reports, invoices) has value. It is the smallest slice that delivers standalone value to an individual user.

**Independent Test**: With a seeded project and task, a signed-in user starts a timer, stops it, adds one manual entry, edits an entry's notes, and sees a correct running total for the day — all without any client/invoice setup beyond the seed.

**Acceptance Scenarios**:

1. **Given** a signed-in user with no running timer, **When** they start a timer on a project/task, **Then** a running entry appears and its elapsed time increments live.
1. **Given** a user with a running timer, **When** they stop it, **Then** the entry is saved with a start time, end time, and computed duration, and no timer is running.
1. **Given** a user, **When** they add a manual entry with a date, duration, project, task, and notes, **Then** it is saved and appears in their list of entries.
1. **Given** a saved entry, **When** the user edits its duration, notes, or billable flag, **Then** the changes persist and totals update accordingly.
1. **Given** a user with a running timer, **When** they attempt to start a second timer, **Then** the system prevents two concurrent running timers (the first is stopped or the action is blocked with a clear message).

______________________________________________________________________

### User Story 2 - Organize clients, projects, and tasks (Priority: P2)

A manager sets up the billing structure: they add clients, create projects under a client (with a billing method, optional budget, and rate), and define tasks within a project. This gives everyone a correct, consistent set of things to track time against.

**Why this priority**: Time must attach to meaningful work items with correct billing attributes; this structure is a prerequisite for billing and reporting, but is only needed once tracking (P1) exists.

**Independent Test**: A manager creates a client, a project under it (with a billing method and rate), and a task under the project; the new task then becomes selectable when tracking time.

**Acceptance Scenarios**:

1. **Given** a manager, **When** they create a client with a name and currency, **Then** the client is saved and available for projects.
1. **Given** a client, **When** the manager creates a project with a billing method (e.g., time-and-materials, fixed fee, non-billable, retainer), an optional budget, and a rate, **Then** the project is saved under that client.
1. **Given** a project, **When** the manager adds a task with an optional rate and a billable flag, **Then** the task is selectable when logging time on that project.
1. **Given** an active project, **When** the manager marks it inactive, **Then** it stops appearing as an option for new time entries while existing entries are preserved.

______________________________________________________________________

### User Story 3 - Invoice tracked time (Priority: P3)

A manager selects a client and a period, reviews the billable, not-yet-invoiced time for that client, and generates an invoice with line items derived from that time. They move the invoice through its lifecycle (draft → sent → paid, or void) and export it.

**Why this priority**: Converting tracked time into billing is the primary business outcome, but it depends on both tracked time (P1) and billing structure (P2) already existing.

**Independent Test**: With billable time recorded for a client, a manager generates a draft invoice for a period, sees line items and a total that match the underlying time, marks it sent then paid, and exports it.

**Acceptance Scenarios**:

1. **Given** billable, un-invoiced time for a client in a period, **When** the manager generates an invoice, **Then** a draft invoice is created with line items and a total that reconcile exactly with the selected time entries.
1. **Given** a generated invoice, **When** it is created, **Then** the included time entries are marked as invoiced so they cannot be billed again.
1. **Given** a draft invoice, **When** the manager marks it sent and later paid (or void), **Then** its status updates and is reflected in invoice lists.
1. **Given** a finalized invoice, **When** the manager exports it, **Then** the exported document's amounts match the on-screen invoice exactly.
1. **Given** a draft invoice, **When** the manager adjusts its editable fields and generates the PDF, **Then** a branded, print-ready PDF is produced whose amounts reconcile exactly with the invoice, and regenerating it yields the same document (FR-025).

______________________________________________________________________

### User Story 4 - Administer users and access (Priority: P4)

An administrator provisions user accounts, assigns each a role (administrator, manager, or member), and deactivates people who leave. Roles determine what each person can see and do.

**Why this priority**: Access control is necessary for a multi-person deployment and to protect billing data, but a single seeded admin is enough to exercise P1–P3 first.

**Independent Test**: An administrator creates a member and a manager account, signs in as each, and confirms the member cannot manage clients/invoices while the manager can; deactivating a user prevents them from signing in.

**Acceptance Scenarios**:

1. **Given** an administrator, **When** they create a user with a role, **Then** that user can sign in and is limited to the permissions of their role.
1. **Given** a member (non-manager), **When** they attempt to manage clients, projects, or invoices, **Then** the action is not available to them.
1. **Given** an active user, **When** an administrator deactivates them, **Then** that user can no longer sign in and their historical entries remain intact.

______________________________________________________________________

### User Story 5 - Extend with plugins (Priority: P5)

An operator drops a sandboxed plugin into the deployment to react to business events (for example, post to a chat channel when an invoice is sent) or to surface an extra widget on the dashboard — without changing the core application.

**Why this priority**: Extensibility is a differentiator and enables integrations, but the product is fully usable without any plugins installed.

**Independent Test**: An operator installs a sample plugin subscribed to "invoice sent"; sending an invoice triggers the plugin's action (observable via its effect or log), and a plugin that errors or hangs does not block or corrupt the invoice action.

**Acceptance Scenarios**:

1. **Given** a plugin declaring the events it handles, **When** the deployment starts, **Then** the plugin is loaded and registered for those events.
1. **Given** a loaded plugin subscribed to an event, **When** that event occurs (time entry created/stopped, invoice created/sent, user signed in), **Then** the plugin is invoked with the event's data.
1. **Given** a plugin that fails, times out, or attempts a disallowed action, **When** its event fires, **Then** the core action still completes and the failure is isolated and logged.
1. **Given** a plugin that returns a dashboard widget, **When** a user views the dashboard, **Then** the widget's content is displayed in a designated plugin area.

### Edge Cases

- A timer left running across midnight or for an unusually long period is still recorded with an exact duration.
- Attempting to invoice a period with no billable un-invoiced time produces a clear "nothing to invoice" result rather than an empty invoice.
- Editing or deleting a time entry that has already been included in an invoice is prevented (or requires removing it from the invoice / voiding the invoice first).
- Deactivating a user or deactivating a client/project that has historical time or invoices preserves that history and does not orphan records.
- Two people tracking against the same task concurrently each get independent entries and correct individual and combined totals.
- A malformed, unsupported, or malicious plugin is rejected at load time and never gains capabilities beyond those explicitly granted.

## Requirements *(mandatory)*

### Functional Requirements

**Access & accounts**

- **FR-001**: The system MUST require users to authenticate before accessing any data, and MUST restrict actions by role (administrator, manager, member).
- **FR-002**: Administrators MUST be able to create user accounts, assign roles, and deactivate accounts; deactivated users MUST be unable to sign in.

**Time tracking**

- **FR-003**: Users MUST be able to start and stop a timer against a specific project and task, producing an entry with an exact start time, end time, and duration.
- **FR-004**: The system MUST prevent a single user from having more than one running timer at a time.
- **FR-005**: Users MUST be able to create, edit, and delete manual time entries (date, duration, project, task, notes, billable flag) for their own time.
- **FR-006**: Each time entry MUST record whether it is billable and MUST be attributable to exactly one user, project, and task.
- **FR-007**: Users MUST be able to view and filter their time entries and see accurate per-day and per-period totals.

**Clients, projects, tasks**

- **FR-008**: Managers MUST be able to create, edit, and deactivate clients, each with a name and a currency.
- **FR-009**: Managers MUST be able to create projects under a client with a billing method (time-and-materials, fixed fee, non-billable, or retainer), an optional budget, and a rate.
- **FR-010**: Managers MUST be able to create tasks under a project, each with an optional rate and a billable flag.
- **FR-011**: Inactive clients, projects, and tasks MUST NOT be selectable for new time entries while remaining associated with existing entries.

**Invoicing**

- **FR-012**: Managers MUST be able to generate an invoice for a client from that client's billable, un-invoiced time within a selected period, with line items and a total that reconcile exactly with the underlying entries.
- **FR-013**: The system MUST mark time entries as invoiced when they are included on an invoice and MUST prevent the same time from being billed on more than one invoice.
- **FR-014**: Invoices MUST support the lifecycle states draft, sent, paid, and void, and MUST carry an invoice number, issue date, due date, and total.
- **FR-015**: The system MUST prevent editing or deleting time that is already attached to an invoice unless it is first removed from that invoice.
- **FR-025**: The system MUST render each invoice as a print-ready PDF whose line items and totals reconcile exactly with the invoice (FR-012/FR-023). The document MUST be produced from a customizable template (branding, layout, and embeddable fonts), MUST be reproducible (the same invoice yields the same document), and users MUST be able to review and adjust the invoice's editable fields before it is finalized or sent.

**Reporting & export**

- **FR-016**: Managers MUST be able to produce reports of tracked time grouped by client, project, task, or user for a chosen period, and export them in a portable format whose totals reconcile exactly with the on-screen figures.
- **FR-017**: Users MUST see a dashboard summarizing their recent activity (for example, the current period's hours and any running timer).

**Extensibility (plugins)**

- **FR-018**: The system MUST load operator-provided plugins at startup and register each for the events it declares.
- **FR-019**: The system MUST dispatch defined business events — time entry created, time entry stopped, invoice created, invoice sent, and user signed in — to all subscribed plugins with the relevant event data.
- **FR-020**: Plugins MUST run in a sandbox limited to explicitly granted capabilities (structured logging, read-only data lookups, outbound network calls, and access to their own configuration) and MUST NOT be able to modify stored data directly.
- **FR-021**: A plugin that fails, times out, or exceeds its permissions MUST NOT block, unduly delay, or corrupt the core action that triggered it; failures MUST be isolated and logged.
- **FR-022**: Plugins MUST be able to contribute structured dashboard widgets that the system renders in designated plugin areas; plugins MUST NOT be able to inject arbitrary interface code.

**Correctness**

- **FR-023**: All time and monetary totals MUST be computed exactly, with no rounding drift, so that any total equals the sum of its parts across any grouping or period.
- **FR-024**: When billing a time entry, the system MUST resolve the applicable rate deterministically, in this order: (1) the task's rate on that project, if set; (2) the user's per-project assignment rate override, if set; (3) the project's rate; (4) the user's default billable rate. The resolved rate × the entry's billable minutes MUST determine the line amount.

### Key Entities *(include if feature involves data)*

- **Organization**: The single tenant that owns all data (one per deployment); holds defaults such as currency and week start.
- **User**: A person who signs in; has a display name, a role (administrator/manager/member), and an active/inactive status; owns time entries.
- **Client**: An organization being billed; has a name and a currency; owns projects.
- **Project**: A body of work for a client; has a name/code, a billing method, an optional budget, and a rate; owns tasks and time entries.
- **Task**: A unit of work within a project; has an optional rate and a billable flag.
- **Assignment**: Links a user to a project (with an optional per-project rate override), governing which projects a member may log time against.
- **Time Entry**: A recorded interval or manual duration of work by a user against a project and task; has a start/end (or duration), notes, a billable flag, and an invoiced/not-invoiced state.
- **Invoice**: A bill to a client derived from billable time; has an invoice number, a status (draft/sent/paid/void), issue and due dates, a total, and the set of time entries it covers.
- **Plugin**: An installed, sandboxed extension; declares the events it subscribes to and may provide dashboard widgets; has its own configuration.
- **Event**: A business occurrence (time entry created/stopped, invoice created/sent, user signed in) delivered to subscribed plugins.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A signed-in user can start tracking time against a project in 3 interactions or fewer and under 10 seconds.
- **SC-002**: Reported time and monetary totals reconcile exactly (zero discrepancy) with the sum of underlying entries across any grouping and any reporting period.
- **SC-003**: A manager can generate a client's invoice for a period, from tracked time, in under 2 minutes.
- **SC-004**: At least 95% of time-entry save attempts (timer stop or manual entry) succeed on the first try under normal conditions.
- **SC-005**: With a single-organization deployment holding at least 50 active users and 100,000 time entries, list and report views return within 2 seconds.
- **SC-006**: A subscribed plugin receives an event within 1 second of the triggering action, and a failing or slow plugin never prevents that action from completing successfully.
- **SC-007**: Exported reports and invoices reconcile exactly with their on-screen equivalents.
- **SC-008**: An operator can self-host the application and reach a usable state (initial administrator available, first time entry recordable) by following the documentation, without any external SaaS dependency.

## Assumptions

- **Single organization per deployment**: One tenant per install; multi-organization/multi-tenant support is out of scope for this specification (data is modeled so it could be added later).
- **Self-hosted**: The operator runs the application themselves and provisions the initial administrator account; there is no hosted sign-up flow.
- **Credential-based, role-based access**: Users authenticate with credentials and are authorized by role; the exact authentication mechanism (for example, a local password vs. an external identity provider, plus a local development bypass) is an implementation choice left to the plan phase.
- **No separate timesheet approval workflow in v1**: Billable, un-invoiced time is directly invoiceable. Any approval/submission surface already present in the codebase is out of scope for this feature and is neither required nor removed by it; a formal approve step may be specified later.
- **Currency is per client**: Each client has a single currency and its invoices use it; multi-currency invoices and currency conversion are out of scope. Monetary values are handled as exact minor units and durations as exact units to guarantee SC-002/SC-007.
- **Rate resolution**: Billing rates cascade task → assignment override → project → user default (FR-024); a non-billable task or project yields no billable amount regardless of rate.
- **Plugins are operator-trusted but sandboxed**: The operator chooses which plugins to install; the system still confines each to its granted capabilities. Plugins are portable modules and may be authored in any language that targets the supported sandbox format.
- **Standard export formats**: Reports export to a common spreadsheet/CSV format and invoices to a print-ready PDF (FR-025). Invoice/timesheet documents are rendered from templates with embeddable fonts and deterministic output; the specific rendering engine is an implementation choice (see research.md/plan.md).
- **Web application on modern browsers**: The interface targets current desktop browsers; native mobile apps are out of scope for v1.
