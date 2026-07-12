use chrono::{DateTime, Utc};
use horae_core::types::RoundDir;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub default_currency: String,
    pub week_start: i16,
    pub round_minutes: i16,
    pub round_dir: RoundDir,
    pub created_at: DateTime<Utc>,
    // Branding (FR-025)
    pub provider_name: Option<String>,
    pub provider_address: Option<String>,
    pub provider_tax_id: Option<String>,
    pub provider_email: Option<String>,
    pub provider_phone: Option<String>,
    pub bank_name: Option<String>,
    pub bank_iban: Option<String>,
    pub bank_bic: Option<String>,
    pub bank_routing: Option<String>,
    pub bank_account: Option<String>,
    pub invoice_notes: Option<String>,
    pub invoice_payment_terms: Option<String>,
}

/// Branding-only DTO for invoice rendering and settings UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct OrgBranding {
    pub provider_name: Option<String>,
    pub provider_address: Option<String>,
    pub provider_tax_id: Option<String>,
    pub provider_email: Option<String>,
    pub provider_phone: Option<String>,
    pub bank_name: Option<String>,
    pub bank_iban: Option<String>,
    pub bank_bic: Option<String>,
    pub bank_routing: Option<String>,
    pub bank_account: Option<String>,
    pub invoice_notes: Option<String>,
    pub invoice_payment_terms: Option<String>,
}
