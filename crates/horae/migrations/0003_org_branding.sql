-- Per-organization invoice branding and provider identity (FR-025).
-- All columns nullable so existing orgs are unaffected.

ALTER TABLE organizations
  ADD COLUMN provider_name         text,
  ADD COLUMN provider_address      text,
  ADD COLUMN provider_tax_id       text,
  ADD COLUMN provider_email        text,
  ADD COLUMN provider_phone        text,
  ADD COLUMN bank_name             text,
  ADD COLUMN bank_iban             text,
  ADD COLUMN bank_bic              text,
  ADD COLUMN bank_routing          text,
  ADD COLUMN bank_account          text,
  ADD COLUMN invoice_notes         text,
  ADD COLUMN invoice_payment_terms text DEFAULT 'Net 30';
