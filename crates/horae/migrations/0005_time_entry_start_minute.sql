-- Optional time-of-day for a completed time entry, as minutes since local
-- midnight (0..=1439). NULL = untimed (duration-only): the calendar stacks those
-- from the top of the day as before. A start time positions the entry at its
-- hour. Duration stays in `minutes`; the end is derived (start + minutes) and an
-- entry never crosses midnight.
ALTER TABLE time_entries
  ADD COLUMN start_minute integer;

ALTER TABLE time_entries
  ADD CONSTRAINT time_entries_start_minute_range
    CHECK (start_minute IS NULL OR (start_minute >= 0 AND start_minute <= 1439));

ALTER TABLE time_entries
  ADD CONSTRAINT time_entries_within_day
    CHECK (start_minute IS NULL OR start_minute + minutes <= 1440);
