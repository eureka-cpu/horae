use thiserror::Error;

#[derive(Debug, Error)]
pub enum DurationError {
    #[error("invalid duration format: {0}")]
    InvalidFormat(String),
}

/// Parse "H:MM" or decimal hours (e.g. "1:30" or "1.5") into minutes.
pub fn parse(s: &str) -> Result<u32, DurationError> {
    let s = s.trim();
    if let Some((h, m)) = s.split_once(':') {
        let hours: u32 = h
            .trim()
            .parse()
            .map_err(|_| DurationError::InvalidFormat(s.to_owned()))?;
        let mins: u32 = m
            .trim()
            .parse()
            .map_err(|_| DurationError::InvalidFormat(s.to_owned()))?;
        if mins >= 60 {
            return Err(DurationError::InvalidFormat(s.to_owned()));
        }
        Ok(hours * 60 + mins)
    } else {
        let hours: f64 = s
            .parse()
            .map_err(|_| DurationError::InvalidFormat(s.to_owned()))?;
        Ok((hours * 60.0).round() as u32)
    }
}

/// Format minutes as "H:MM".
pub fn format_hhmm(minutes: u32) -> String {
    format!("{}:{:02}", minutes / 60, minutes % 60)
}

/// Format minutes as decimal hours (e.g. 90 → "1.5").
pub fn format_decimal(minutes: u32) -> String {
    let decimal = minutes as f64 / 60.0;
    if decimal.fract() == 0.0 {
        format!("{}", decimal as u32)
    } else {
        format!("{:.2}", decimal).trim_end_matches('0').to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_colon() {
        assert_eq!(parse("1:30").unwrap(), 90);
        assert_eq!(parse("0:00").unwrap(), 0);
        assert_eq!(parse("2:05").unwrap(), 125);
    }

    #[test]
    fn parse_decimal() {
        assert_eq!(parse("1.5").unwrap(), 90);
        assert_eq!(parse("0.25").unwrap(), 15);
    }

    #[test]
    fn format_round_trip() {
        assert_eq!(format_hhmm(90), "1:30");
        assert_eq!(format_decimal(90), "1.5");
    }
}
