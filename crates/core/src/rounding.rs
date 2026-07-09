use crate::types::RoundDir;

/// Round `minutes` to the nearest multiple of `inc`.
///
/// - `inc == 0` → identity (no rounding)
/// - `RoundDir::Up` → ceiling to next multiple
/// - `RoundDir::Down` → floor to previous multiple
/// - `RoundDir::Nearest` → nearest multiple (ties round up)
pub fn round(minutes: u32, inc: u32, dir: RoundDir) -> u32 {
    if inc == 0 {
        return minutes;
    }
    match dir {
        RoundDir::Up => {
            let rem = minutes % inc;
            if rem == 0 {
                minutes
            } else {
                minutes + (inc - rem)
            }
        }
        RoundDir::Down => minutes - (minutes % inc),
        RoundDir::Nearest => {
            let lower = minutes - (minutes % inc);
            let upper = lower + inc;
            if minutes - lower < upper - minutes {
                lower
            } else {
                upper
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_when_inc_zero() {
        for dir in [RoundDir::Up, RoundDir::Down, RoundDir::Nearest] {
            assert_eq!(round(73, 0, dir), 73);
        }
    }

    #[test]
    fn result_is_multiple_of_inc() {
        for m in 0..=120u32 {
            let r = round(m, 15, RoundDir::Nearest);
            assert_eq!(r % 15, 0, "round({m}, 15, Nearest) = {r}");
        }
    }

    #[test]
    fn nearest_ties_round_up() {
        // 7.5 min → ties between 0 and 15 → rounds up to 15
        assert_eq!(round(8, 15, RoundDir::Nearest), 15);
        assert_eq!(round(7, 15, RoundDir::Nearest), 0);
    }

    #[test]
    fn up_and_down() {
        assert_eq!(round(61, 15, RoundDir::Up), 75);
        assert_eq!(round(61, 15, RoundDir::Down), 60);
        assert_eq!(round(60, 15, RoundDir::Up), 60); // already multiple
    }
}
