/// FR-024 rate resolution cascade.
///
/// Returns the first non-None rate in priority order:
/// 1. Task rate on the project (`project_tasks.rate_cents`)
/// 2. User's per-project assignment override (`assignments.rate_cents`)
/// 3. User's org-wide default (`users.billable_rate_cents`)
pub fn resolve_rate(
    task_rate_cents: Option<i64>,
    assignment_rate_cents: Option<i64>,
    user_rate_cents: Option<i64>,
) -> Option<i64> {
    task_rate_cents
        .or(assignment_rate_cents)
        .or(user_rate_cents)
}

/// Compute line amount in minor units (cents).
///
/// `rate_cents` is an hourly rate in cents; `minutes` is the duration.
/// Uses banker's rounding: `(rate * minutes + 30) / 60` so a half-cent
/// rounds to the nearest even cent.
pub fn line_amount_cents(rate_cents: i64, minutes: i32) -> i64 {
    (rate_cents * minutes as i64 + 30) / 60
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_rate_cascade() {
        // Task rate wins when present
        assert_eq!(resolve_rate(Some(5000), Some(4000), Some(3000)), Some(5000));
        // Falls through to assignment
        assert_eq!(resolve_rate(None, Some(4000), Some(3000)), Some(4000));
        // Falls through to user default
        assert_eq!(resolve_rate(None, None, Some(3000)), Some(3000));
        // All None
        assert_eq!(resolve_rate(None, None, None), None);
    }

    #[test]
    fn line_amount_exact_hour() {
        // $100/hr for 60 minutes = $100.00
        assert_eq!(line_amount_cents(10000, 60), 10000);
    }

    #[test]
    fn line_amount_half_hour() {
        // $100/hr for 30 minutes = $50.00
        assert_eq!(line_amount_cents(10000, 30), 5000);
    }

    #[test]
    fn line_amount_90_minutes() {
        // $100/hr for 90 minutes = $150.00
        assert_eq!(line_amount_cents(10000, 90), 15000);
    }

    #[test]
    fn line_amount_zero_minutes() {
        assert_eq!(line_amount_cents(10000, 0), 0);
    }

    #[test]
    fn line_amount_zero_rate() {
        assert_eq!(line_amount_cents(0, 60), 0);
    }

    #[test]
    fn line_amount_odd_minutes() {
        // $120/hr (12000 cents) for 25 minutes = 12000 * 25 / 60 = 5000
        assert_eq!(line_amount_cents(12000, 25), 5000);
    }
}
