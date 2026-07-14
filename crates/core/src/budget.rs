//! Budget-consumption band arithmetic for the derived plugin budget events.
//!
//! Integer-only (Constitution I): consumption and budget are compared as
//! integers — minutes for `hours` budgets, minor currency units for `amount`
//! budgets — never as floats. Threshold bands are percentages (e.g. 80, 100).
//!
//! The `100%` over-budget line is treated as an implicit band even when it is
//! not among the configured warnings, so exceeding budget is always detectable
//! regardless of how an organization sets its warning thresholds.

/// The effective bands: the configured warning percentages plus the implicit
/// `100` over-budget line, positive-only, sorted and de-duplicated.
fn effective_bands(thresholds: &[i32]) -> Vec<i32> {
    let mut bands: Vec<i32> = thresholds.iter().copied().filter(|&b| b > 0).collect();
    if !bands.contains(&100) {
        bands.push(100);
    }
    bands.sort_unstable();
    bands.dedup();
    bands
}

/// Whether `consumed` has reached `band`% of `budget`, in the cross-multiplied
/// integer form (`consumed * 100 >= budget * band`) — no division, no floats.
/// The `i128` widening avoids overflow near the `i64` bounds.
fn reached(consumed: i64, budget: i64, band: i32) -> bool {
    i128::from(consumed) * 100 >= i128::from(budget) * i128::from(band)
}

/// The highest effective band `consumed` has reached against `budget` (`0` when
/// none is reached, or the budget is non-positive). Store this as the project's
/// last-announced band so it advances up and resets down.
pub fn current_band(consumed: i64, budget: i64, thresholds: &[i32]) -> i32 {
    if budget <= 0 {
        return 0;
    }
    effective_bands(thresholds)
        .into_iter()
        .filter(|&b| reached(consumed, budget, b))
        .max()
        .unwrap_or(0)
}

/// Every effective band newly crossed since `last_band`, in ascending order —
/// one event per band. Includes the `100` over-budget line even when it is not a
/// configured warning. Empty on a no-op, a drop in consumption, or a
/// non-positive budget, so each crossing is announced at most once.
pub fn newly_crossed_bands(
    consumed: i64,
    budget: i64,
    thresholds: &[i32],
    last_band: i32,
) -> Vec<i32> {
    if budget <= 0 {
        return Vec::new();
    }
    effective_bands(thresholds)
        .into_iter()
        .filter(|&b| b > last_band && reached(consumed, budget, b))
        .collect()
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
        assert!(newly_crossed_bands(500, 0, BANDS, 0).is_empty());
    }

    #[test]
    fn each_crossing_is_announced_once() {
        // First reach of 80% announces just 80.
        assert_eq!(newly_crossed_bands(80, 100, BANDS, 0), vec![80]);
        // Still inside the 80 band — nothing new.
        assert!(newly_crossed_bands(95, 100, BANDS, 80).is_empty());
        // Reaching 100% announces 100.
        assert_eq!(newly_crossed_bands(100, 100, BANDS, 80), vec![100]);
        // Already at 100 — nothing new.
        assert!(newly_crossed_bands(140, 100, BANDS, 100).is_empty());
    }

    #[test]
    fn a_single_jump_announces_every_crossed_band() {
        // 0% -> 150% in one write crosses both 80 and 100.
        assert_eq!(newly_crossed_bands(150, 100, BANDS, 0), vec![80, 100]);
    }

    #[test]
    fn over_budget_fires_even_without_a_configured_100_band() {
        // Org only warns at 80%; a 150% overrun must still cross the implicit 100.
        let only_80: &[i32] = &[80];
        assert_eq!(newly_crossed_bands(150, 100, only_80, 0), vec![80, 100]);
        assert_eq!(current_band(150, 100, only_80), 100);
        // And exactly at budget with no bands configured at all still reports 100.
        assert_eq!(newly_crossed_bands(100, 100, &[], 0), vec![100]);
    }

    #[test]
    fn drop_below_a_band_does_not_fire_but_current_resets() {
        assert!(newly_crossed_bands(50, 100, BANDS, 100).is_empty());
        assert_eq!(current_band(50, 100, BANDS), 0);
        assert_eq!(newly_crossed_bands(85, 100, BANDS, 0), vec![80]);
    }

    #[test]
    fn large_values_do_not_overflow() {
        // consumed * 100 would overflow i64 near i64::MAX; i128 widening is safe.
        assert_eq!(current_band(i64::MAX, i64::MAX, BANDS), 100);
    }
}
