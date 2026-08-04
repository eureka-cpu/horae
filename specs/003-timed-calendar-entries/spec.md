# Feature Specification: Timed Calendar Entries

**Feature Branch**: `003-timed-calendar-entries`

**Created**: 2026-08-03

**Status**: Draft

**Input**: User description: "Add an optional start time (time of day) to time entries so the Timesheet Calendar view can place entries at their real time instead of stacking by duration, and let users drag on the calendar grid to create a new entry spanning a time slot. Entries created without a start time keep working and stack as today. Durations stay in whole minutes; the start time is an optional time of day."

## Clarifications

### Session 2026-08-03

- Q: Besides creating by dragging an empty slot, should calendar dragging also move and resize existing timed entries? → A: Yes — create + resize + move, matching Harvest, prioritized so create is the MVP (P1) and resize/move follow.
- Q: Where do entries without a start time appear on the calendar? → A: Stacked from the top of the day sized by duration (matching Harvest and the current behavior), not a separate untimed lane.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Draw a time entry on the calendar (Priority: P1)

A person opens the Timesheet in Calendar view and presses on the grid at, say, 9:00 in Tuesday's column, drags down to 11:30, and releases. A block appears spanning 9:00–11:30 and an entry form opens pre-filled with that day, a start time of 9:00, and a duration of 2:30. They pick a project and task and save. The entry stays on the calendar at 9:00, two-and-a-half hours tall.

**Why this priority**: This is the headline interaction users expect from a calendar timesheet and the whole reason to add a time of day. It turns time entry from typing durations into sketching your day, matching the tool users are migrating from.

**Independent Test**: Open Calendar view, drag across a slot on one day, confirm the form opens with the dragged day/start/duration, save, and confirm the entry renders at the dragged position and persists after reload.

**Acceptance Scenarios**:

1. **Given** an empty Tuesday in Calendar view, **When** the user drags from 9:00 to 11:30 in Tuesday's column and releases, **Then** an entry form opens pre-filled with Tuesday, start 9:00, duration 2:30.
1. **Given** the pre-filled form, **When** the user selects a project and task and saves, **Then** a new entry is created for Tuesday starting at 9:00 lasting 2:30, and it renders at that position on the calendar.
1. **Given** a saved timed entry, **When** the user reloads or shares the timesheet URL, **Then** the entry still appears at its start time with the same duration.
1. **Given** a drag that ends higher than it began, **When** the user releases, **Then** the slot is interpreted as start = the earlier time and duration = the absolute difference (drag direction does not matter).

______________________________________________________________________

### User Story 2 - See entries at the time they happened (Priority: P2)

A person who has recorded (or run a timer for) work at specific times opens Calendar view and sees each timed entry placed at its real hour, so the day reads like a schedule — morning blocks near the top, afternoon lower — rather than a single stack whose height is just the total.

**Why this priority**: Placement by real time is what makes the calendar meaningful; without it, a start time is stored but never seen. It also makes overlapping/adjacent work legible at a glance.

**Independent Test**: Give one day two entries with different start times, open Calendar view, and confirm each renders at its own time position (not stacked from the top by cumulative duration).

**Acceptance Scenarios**:

1. **Given** an entry that starts at 14:00 for 1:00, **When** the user views that day in Calendar view, **Then** the entry's block begins at the 14:00 gridline and is one hour tall.
1. **Given** a running timer that was started at a real clock time, **When** the timer is stopped, **Then** the resulting entry records that clock time as its start time and appears there on the calendar.
1. **Given** two timed entries whose times overlap, **When** the user views the day, **Then** both entries remain visible and readable (neither is hidden or lost).

______________________________________________________________________

### User Story 3 - Keep untimed entries working (Priority: P3)

A person continues to add time the quick way — typing a duration in the Week grid, or using the "+" form without touching a time — and everything keeps working. Those entries have no time of day; on the calendar they stack from the top of the day sized by duration (the current behavior, which the source tool also uses for duration-only entries), and their day/week totals are unchanged.

**Why this priority**: The majority of existing entries and the fastest entry paths have no time of day. The feature must be strictly additive — it cannot force anyone to pick a time or move existing data.

**Independent Test**: Add an entry via the Week grid (duration only), open Calendar view, and confirm the entry stacks from the top of its day and all totals are unchanged; confirm existing entries created before this feature still display and edit normally.

**Acceptance Scenarios**:

1. **Given** an entry with a duration but no start time, **When** the user views Calendar view, **Then** the entry stacks from the top of its day sized by its duration and counts toward the day and week totals.
1. **Given** a timed entry, **When** the user clears its start time in the entry form, **Then** it becomes an untimed entry and stacks from the top of the day with its duration unchanged.
1. **Given** entries that existed before this feature, **When** the user opens any timesheet view, **Then** they display, edit, submit, and total exactly as before.

______________________________________________________________________

### User Story 4 - Adjust a timed entry by dragging (Priority: P3)

Once entries sit on the calendar at their time, the user drags an entry's edge to make it longer or shorter, or drags the whole block to a different hour (or day), the way a calendar app lets you reshape events — without opening a form. The start time and duration update to match where they let go. This matches the source tool (create, move, and resize are all drag gestures there).

**Why this priority**: Move and resize round out the drag experience and match Harvest, but they are refinements on top of create + display; the feature is already useful without them, so they come after the MVP.

**Independent Test**: With a timed entry on the calendar, drag its bottom edge and confirm its duration changes; drag its body to another hour/day and confirm its start time (and date) change; confirm both persist after reload.

**Acceptance Scenarios**:

1. **Given** a timed entry lasting 1:00, **When** the user drags its bottom edge down by one hour and releases, **Then** its duration becomes 2:00 with the same start time.
1. **Given** a timed entry at 09:00, **When** the user drags its body to 13:00 on the same day, **Then** its start time becomes 13:00 with the same duration.
1. **Given** a timed entry on Monday, **When** the user drags it into Wednesday's column, **Then** it moves to Wednesday keeping its start time and duration.
1. **Given** a locked (submitted/approved) entry, **When** the user tries to move or resize it, **Then** it does not move and the user is told it is locked.

______________________________________________________________________

### Edge Cases

- **Drag past end of day**: a drag (or a start time plus duration) that would extend past midnight is clamped so the entry ends at the end of the same day; entries never span two calendar days.
- **Zero-length drag / a plain click**: a press-and-release with no meaningful movement opens the entry form for that day with no start time and the usual empty duration (identical to today's "click a day" behavior).
- **Very short drag**: a drag shorter than one snap step is treated as the minimum entry length (one snap step) rather than a zero-duration entry.
- **Overlapping entries**: multiple timed entries at the same hour are all shown without hiding any; totals still equal the exact sum of every entry.
- **Editing duration of a timed entry**: changing the duration keeps the start time and re-sizes the block from that start.
- **Locked entries** (submitted/approved): a timed entry that is locked cannot be moved or resized, consistent with existing edit rules.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A time entry MUST support an optional start time expressed as a precise time of day on its existing calendar date. Whether or not a start time is present, the entry's duration remains an exact whole-minute value.
- **FR-002**: Users MUST be able to create a time entry by pressing on the Calendar grid at a start position, dragging to an end position within the same day, and releasing; the system MUST interpret the earlier position as the start time and the absolute distance as the duration.
- **FR-003**: On release of such a drag, the system MUST open the entry form pre-filled with the dragged day, start time, and duration so the user can choose project/task (and notes) before the entry is saved.
- **FR-004**: While dragging, the system MUST show a live preview block that grows and shrinks with the pointer so the user sees the slot they are about to create.
- **FR-005**: The Calendar view MUST place each entry that has a start time at that time on the day's grid, with a height proportional to its duration.
- **FR-006**: The Calendar view MUST present entries that have no start time stacked from the top of their day, sized by duration — the current behavior, which the source tool also uses for duration-only entries — and MUST keep them included in all totals.
- **FR-007**: Users MUST be able to set, change, or clear an entry's start time from the entry form; clearing it converts the entry to untimed without changing its duration.
- **FR-008**: The system MUST snap dragged and entered start times and durations to a consistent, small time increment so entries land on tidy boundaries.
- **FR-009**: When a running timer is stopped, the resulting entry MUST record the real clock time at which the timer started as its start time.
- **FR-010**: Existing entries, and any entry created without a start time (e.g. via the Week grid or the plain "+" form), MUST continue to be created, displayed, edited, submitted, invoiced, and totaled exactly as before this feature.
- **FR-011**: Day and week totals, and any billing or approval behavior, MUST be unaffected by the presence or absence of a start time; a start time is display/scheduling metadata only and MUST NOT change any monetary or duration total.
- **FR-012**: An entry's start time plus duration MUST NOT cross midnight; the system MUST prevent or clamp inputs that would extend an entry past the end of its day.
- **FR-013**: Moving or resizing a timed entry MUST be blocked when the entry is in a locked state (submitted/approved), consistent with existing edit restrictions.
- **FR-014**: Users MUST be able to resize a timed entry by dragging its edge on the calendar — the bottom edge changes its duration; the top edge changes its start time (and duration) — snapped to the standard increment.
- **FR-015**: Users MUST be able to move a timed entry by dragging its body to a different time on the same day (changing its start time) or to a different day (changing its date), keeping its duration. Priority note: create (FR-002–FR-004) is the MVP; resize (FR-014) and move (FR-015) are a follow-up phase.

### Key Entities *(include if feature involves data)*

- **Time Entry**: an amount of work on a given date for a project/task. Gains one new, **optional (nullable)** attribute: a **start time** — a precise time of day within that date, mirroring the source tool's nullable start-of-day field (null for duration-only entries). All existing attributes — date, duration in whole minutes, notes, billable flag, state — are unchanged. An entry is "timed" when it has a start time and "untimed" when it does not.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can create a fully-specified timed entry (project, task, start, duration) by dragging on the calendar and confirming the form in under 15 seconds.
- **SC-002**: 100% of entries created by dragging appear on the calendar at the exact slot that was dragged, before and after a page reload.
- **SC-003**: For any day and week, the displayed totals equal the exact sum of every entry's minutes regardless of how many entries are timed vs untimed (no drift, no double counting).
- **SC-004**: 100% of entries that existed before the feature continue to display, edit, submit, and total correctly, with no data migration errors and no forced time-of-day.
- **SC-005**: When two or more entries overlap in time on the same day, all of them remain visible and openable; none are hidden or lost.
- **SC-006**: Stopping a timer produces an entry whose start time matches, to the snap increment, the clock time the timer was started.

## Assumptions

- **Optional per entry, no account-wide mode**: a start time is optional on each entry; there is no separate account setting that forces "start/end time" vs "duration" tracking. This matches the source tool's API, where the start-of-day field is a nullable time (present only when tracking by start/end times), and keeps the feature additive and simpler than a global mode toggle.
- **End time is derived, not stored**: only a start time is added; the end is start + duration. Duration remains the single source of truth for length, preserving whole-minute exactness and all existing totals/billing.
- **Snap increment**: dragged and entered times/durations snap to 15-minute increments, a common scheduling default; the exact value can be tuned during planning.
- **Untimed entries stack from the top**: entries without a start time keep the current calendar behavior — stacked from the top of the day sized by duration — which is also how the source tool shows duration-only entries. No separate untimed lane is introduced.
- **Overlap layout**: overlapping timed entries are laid out so all remain visible (e.g. side by side); exact layout is a design detail.
- **Single-day entries**: an entry belongs to exactly one date and never spans midnight; long sessions crossing midnight are recorded as separate entries per day (unchanged from today).
- **Time zone**: start times are interpreted in the organization's existing working time zone used elsewhere in the timesheet; no new per-user time-zone handling is introduced by this feature.
- **Scope**: this feature targets the Timesheet Day/Week/Calendar experience. It does not add expense tracking, estimates, external calendar import, or any item listed as out of scope in the product spec.

## Dependencies

- Builds on the existing Timesheet (Day/Week/Calendar views), the time-entry create/edit form, and the running-timer feature.
- Relies on the existing week/day navigation and shareable timesheet URLs so a timed calendar is viewable and linkable per week.
