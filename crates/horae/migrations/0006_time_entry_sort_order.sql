-- Explicit ordering for untimed (duration-only) entries within a day so they can
-- be reordered on the calendar. Timed entries are ordered by their start time, so
-- this only matters for the untimed stack. Existing rows default to 0 and keep
-- their created_at order until reordered.
ALTER TABLE time_entries ADD COLUMN sort_order integer NOT NULL DEFAULT 0;
