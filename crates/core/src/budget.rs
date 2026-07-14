//! Budget-consumption band arithmetic for the derived plugin budget events.
//!
//! Integer-only (Constitution I): consumption and budget are compared as
//! integers — minutes for `hours` budgets, minor currency units for `amount`
//! budgets — never as floats. Threshold bands are percentages (e.g. 80, 100).

/// The highest threshold band `consumed` has reached against `budget`, or `0`
/// when none is reached (also `0` if `budget` is non-positive). Uses the
/// cross-multiplied form `consumed * 100 >= budget * band` so no division or
/// floating point is involved; the widening to `i128` avoids overflow.
pub fn current_band(consumed: i64, budget: i64, thresholds: &[i32]) -> i32 {
    if budget <= 0 {
        return 0;
    }
    thresholds
        .iter()
        .copied()
        .filter(|&band| i128::from(consumed) * 100 >= i128::from(budget) * i128::from(band))
        .max()
        .unwrap_or(0)
}

/// The band to announce as newly crossed, given the highest band already
/// announced (`last_band`). `Some(band)` only when consumption has reached a
/// *higher* band than before, so each crossing fires at most once and neither a
/// no-op nor a drop in consumption emits an event.
pub fn crossed_band(consumed: i64, budget: i64, thresholds: &[i32], last_band: i32) -> Option<i32> {
    let current = current_band(consumed, budget, thresholds);
    (current > last_band).then_some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BANDS: &[i32] = &[80, 100];

    #[test]
    fn current_band_is_the_highest_reached() {
        assert_eq!(current_band(79, 100, BANDS), 0);
        assert_eq!(current_band(80, 100, BANDS), 80); // exactly at the band counts
        assert_eq!(current_band(99, 100, BANDS), 80);
        assert_eq!(current_band(100, 100, BANDS), 100);
        assert_eq!(current_band(150, 100, BANDS), 100);
    }

    #[test]
    fn no_band_for_zero_or_missing_budget() {
        assert_eq!(current_band(500, 0, BANDS), 0);
        assert_eq!(current_band(500, -1, BANDS), 0);
        assert_eq!(crossed_band(500, 0, BANDS, 0), None);
    }

    #[test]
    fn crossed_fires_once_per_higher_band() {
        // Crossing 80% for the first time announces 80.
        assert_eq!(crossed_band(80, 100, BANDS, 0), Some(80));
        // Still within the 80 band — nothing new.
        assert_eq!(crossed_band(95, 100, BANDS, 80), None);
        // Crossing 100% announces 100.
        assert_eq!(crossed_band(100, 100, BANDS, 80), Some(100));
        // Already at 100 — nothing new.
        assert_eq!(crossed_band(140, 100, BANDS, 100), None);
    }

    #[test]
    fn drop_below_a_band_does_not_fire_but_current_resets() {
        // Consumption fell back under 80 after having announced 100.
        assert_eq!(crossed_band(50, 100, BANDS, 100), None);
        // The caller stores current_band (0 here) so a later rise re-fires.
        assert_eq!(current_band(50, 100, BANDS), 0);
        assert_eq!(crossed_band(85, 100, BANDS, 0), Some(80));
    }

    #[test]
    fn large_values_do_not_overflow() {
        // consumed * 100 would overflow i64 near i64::MAX; i128 widening is safe.
        assert_eq!(current_band(i64::MAX, i64::MAX, BANDS), 100);
    }
}
