-- Support for the derived plugin events (feature 002 / US3).

-- Organization-level, admin-configurable thresholds shared across plugins.
ALTER TABLE organizations
  ADD COLUMN budget_alert_pcts integer[] NOT NULL DEFAULT '{80,100}',
  ADD COLUMN long_timer_minutes integer NOT NULL DEFAULT 480;

-- Highest budget band already announced for a project, so each threshold
-- crossing fires at most once; reset downward when consumption falls.
ALTER TABLE projects
  ADD COLUMN last_budget_alert_pct integer;

-- Set when timer_running_too_long fired for a running entry; cleared on stop,
-- so a forgotten timer is announced at most once per overrun.
ALTER TABLE time_entries
  ADD COLUMN notified_long_running_at timestamptz;
