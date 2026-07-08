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
