use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money {
    pub cents: i64,
    pub currency: [u8; 3], // ISO 4217 e.g. b"USD"
}

#[derive(Debug, Error)]
#[error("currency mismatch: cannot add {a} and {b}")]
pub struct CurrencyMismatch {
    pub a: String,
    pub b: String,
}

impl Money {
    pub fn new(cents: i64, currency: [u8; 3]) -> Self {
        Self { cents, currency }
    }

    pub fn currency_str(&self) -> &str {
        std::str::from_utf8(&self.currency).unwrap_or("???")
    }
}

pub fn add(a: Money, b: Money) -> Result<Money, CurrencyMismatch> {
    if a.currency != b.currency {
        return Err(CurrencyMismatch {
            a: a.currency_str().to_owned(),
            b: b.currency_str().to_owned(),
        });
    }
    Ok(Money {
        cents: a.cents + b.cents,
        currency: a.currency,
    })
}

/// Format minor units for display: currency code + thousands-grouped decimal,
/// e.g. `format_cents(1_000_000, "USD")` → `"USD 10,000.00"`. A negative amount
/// keeps its sign after the code (`"USD -500.00"`).
pub fn format_cents(cents: i64, currency: &str) -> String {
    let neg = cents < 0;
    let abs = cents.unsigned_abs();
    let whole = (abs / 100).to_string();
    let len = whole.len();
    let mut grouped = String::new();
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    format!(
        "{} {}{}.{:02}",
        currency.trim(),
        if neg { "-" } else { "" },
        grouped,
        abs % 100
    )
}

/// Display a project budget in its own unit: money for an amount budget, hours
/// for an hours budget, empty for no budget.
pub fn format_budget(
    kind: crate::types::BudgetKind,
    amount_cents: Option<i64>,
    minutes: Option<i64>,
    currency: &str,
) -> String {
    use crate::types::BudgetKind;
    match kind {
        BudgetKind::Amount => amount_cents
            .map(|c| format_cents(c, currency))
            .unwrap_or_default(),
        BudgetKind::Hours => minutes
            .map(|m| format!("{}h", crate::duration::format_decimal(m.max(0) as u32)))
            .unwrap_or_default(),
        BudgetKind::None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_cents_groups_thousands_and_keeps_sign() {
        assert_eq!(format_cents(1_000_000, "USD"), "USD 10,000.00");
        assert_eq!(format_cents(150_000, "USD"), "USD 1,500.00");
        assert_eq!(format_cents(99, "USD"), "USD 0.99");
        assert_eq!(format_cents(0, "EUR"), "EUR 0.00");
        assert_eq!(format_cents(-50_000, "USD"), "USD -500.00");
    }
}
