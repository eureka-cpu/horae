//! Time-of-day helpers. An entry's optional start time is **minutes since local
//! midnight** (`0..=1439`). Durations stay whole minutes; the end is
//! `start + duration` and an entry never crosses midnight. All exact integer
//! math — no floats, no I/O (Constitution: Exactness, Domain Purity).

/// Minutes in a day.
pub const DAY_MINUTES: u16 = 24 * 60; // 1440

/// Snap/step granularity for dragged and entered times and durations.
pub const SNAP_STEP: u16 = 15;

/// Smallest duration a drag produces (one snap step, never zero).
pub const MIN_DURATION: u16 = SNAP_STEP;

/// Parse a time of day into minutes since midnight (`0..=1439`).
///
/// Accepts 24h (`"13:30"`, `"9:00"`), 12h (`"1:30pm"`, `"9:00 am"`), and
/// hour-only 12h (`"9am"`). Returns `None` for anything out of range or
/// unparseable.
pub fn parse(s: &str) -> Option<u16> {
    let s = s.trim().to_ascii_lowercase();
    if s.is_empty() {
        return None;
    }
    let (body, meridiem) = if let Some(rest) = s.strip_suffix("am") {
        (rest.trim(), Some(false))
    } else if let Some(rest) = s.strip_suffix("pm") {
        (rest.trim(), Some(true))
    } else {
        (s.as_str(), None)
    };

    let (h_str, m_str) = match body.split_once(':') {
        Some((h, m)) => (h.trim(), m.trim()),
        None => (body, "0"),
    };
    let hours: u16 = h_str.parse().ok()?;
    let mins: u16 = m_str.parse().ok()?;
    if mins >= 60 {
        return None;
    }

    let hours = match meridiem {
        // 12h: hour must be 1..=12; 12am = 0, 12pm = 12, else +12 for pm.
        Some(is_pm) => {
            if !(1..=12).contains(&hours) {
                return None;
            }
            match (is_pm, hours) {
                (false, 12) => 0,
                (false, h) => h,
                (true, 12) => 12,
                (true, h) => h + 12,
            }
        }
        // 24h: hour must be 0..=23.
        None => {
            if hours >= 24 {
                return None;
            }
            hours
        }
    };

    let total = hours * 60 + mins;
    (total < DAY_MINUTES).then_some(total)
}

/// Format minutes-since-midnight as 24h `"H:MM"`.
pub fn format(minutes: u16) -> String {
    format!("{}:{:02}", minutes / 60, minutes % 60)
}

/// Format minutes-since-midnight as 12h `"h:MMam"`/`"h:MMpm"` (Harvest style).
/// Accepts `0..=1440`; `1440` (end of day) wraps to `12:00am`.
pub fn format_12h(minutes: u16) -> String {
    let minutes = minutes % DAY_MINUTES;
    let (h24, m) = (minutes / 60, minutes % 60);
    let (h12, mer) = match h24 {
        0 => (12, "am"),
        1..=11 => (h24, "am"),
        12 => (12, "pm"),
        _ => (h24 - 12, "pm"),
    };
    format!("{h12}:{m:02}{mer}")
}

/// Round a minute value to the nearest `step`, clamped to `>= 0`.
pub fn snap(value: i32, step: u16) -> i32 {
    let step = step as i32;
    if step <= 0 {
        return value.max(0);
    }
    let v = value.max(0);
    ((v + step / 2) / step) * step
}

/// Shrink `duration` so `start + duration <= DAY_MINUTES` — the no-cross-midnight
/// rule. `start` is clamped into range first.
pub fn clamp_to_day(start: u16, duration: u32) -> u32 {
    let start = start.min(DAY_MINUTES);
    let remaining = u32::from(DAY_MINUTES - start);
    duration.min(remaining)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reads_24h() {
        assert_eq!(parse("13:30"), Some(13 * 60 + 30));
        assert_eq!(parse("0:00"), Some(0));
        assert_eq!(parse("9:05"), Some(9 * 60 + 5));
    }

    #[test]
    fn parse_reads_12h_including_edges() {
        assert_eq!(parse("12:00am"), Some(0));
        assert_eq!(parse("12:00pm"), Some(12 * 60));
        assert_eq!(parse("1:30pm"), Some(13 * 60 + 30));
        assert_eq!(parse("9 am"), Some(9 * 60));
        assert_eq!(
            parse("11:59 PM".to_lowercase().as_str()),
            Some(23 * 60 + 59)
        );
    }

    #[test]
    fn parse_rejects_out_of_range_and_garbage() {
        assert_eq!(parse("24:00"), None);
        assert_eq!(parse("9:60"), None);
        assert_eq!(parse("13:00pm"), None);
        assert_eq!(parse("abc"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn format_round_trips() {
        for m in [0u16, 5, 65, 13 * 60 + 30, 1439] {
            assert_eq!(parse(&format(m)), Some(m));
        }
    }

    #[test]
    fn format_12h_matches_harvest_style() {
        assert_eq!(format_12h(0), "12:00am");
        assert_eq!(format_12h(9 * 60), "9:00am");
        assert_eq!(format_12h(12 * 60), "12:00pm");
        assert_eq!(format_12h(13 * 60 + 30), "1:30pm");
        assert_eq!(format_12h(DAY_MINUTES), "12:00am"); // end of day wraps
    }

    #[test]
    fn snap_rounds_to_nearest_step() {
        assert_eq!(snap(0, 15), 0);
        assert_eq!(snap(7, 15), 0);
        assert_eq!(snap(8, 15), 15);
        assert_eq!(snap(22, 15), 15);
        assert_eq!(snap(23, 15), 30);
        assert_eq!(snap(-5, 15), 0);
    }

    #[test]
    fn clamp_to_day_prevents_crossing_midnight() {
        assert_eq!(clamp_to_day(0, 60), 60);
        assert_eq!(clamp_to_day(23 * 60, 120), 60); // 23:00 + 2h → clamped to 60
        assert_eq!(clamp_to_day(DAY_MINUTES - 15, 120), 15);
        assert_eq!(clamp_to_day(DAY_MINUTES, 30), 0);
    }
}
