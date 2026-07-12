/// Deterministic invoice PDF rendering via Typst (FR-025).
///
/// The template is embedded at compile time; fonts come from typst-kit.
/// Same inputs always produce byte-identical PDF output.
use typst::foundations::{Dict, IntoValue, Value};
use typst_as_lib::TypstEngine;

use crate::models::{Invoice, InvoiceLine, OrgBranding};

static INVOICE_TEMPLATE: &str = include_str!("../templates/invoice.typ");

fn build_engine() -> TypstEngine<typst_as_lib::TypstTemplateMainFile> {
    TypstEngine::builder()
        .main_file(INVOICE_TEMPLATE)
        .search_fonts_with(
            typst_as_lib::typst_kit_options::TypstKitFontOptions::default()
                .include_embedded_fonts(true),
        )
        .build()
}

fn opt(s: &Option<String>) -> Value {
    match s {
        Some(v) => v.clone().into_value(),
        None => Value::None,
    }
}

fn build_inputs(
    invoice: &Invoice,
    lines: &[InvoiceLine],
    client_name: &str,
    client_address: Option<&str>,
    client_tax_id: Option<&str>,
    branding: &OrgBranding,
) -> Dict {
    let mut dict = Dict::new();

    dict.insert("invoice_number".into(), invoice.number.clone().into_value());
    dict.insert(
        "issued_on".into(),
        invoice.issued_on.to_string().into_value(),
    );
    dict.insert("due_on".into(), invoice.due_on.to_string().into_value());
    dict.insert(
        "currency".into(),
        invoice.currency.trim().to_string().into_value(),
    );
    dict.insert("total_cents".into(), invoice.total_cents.into_value());
    dict.insert("notes".into(), opt(&invoice.notes));

    dict.insert("client_name".into(), client_name.to_string().into_value());
    dict.insert(
        "client_address".into(),
        match client_address {
            Some(a) => a.to_string().into_value(),
            None => Value::None,
        },
    );
    dict.insert(
        "client_tax_id".into(),
        match client_tax_id {
            Some(t) => t.to_string().into_value(),
            None => Value::None,
        },
    );

    dict.insert("provider_name".into(), opt(&branding.provider_name));
    dict.insert("provider_address".into(), opt(&branding.provider_address));
    dict.insert("provider_tax_id".into(), opt(&branding.provider_tax_id));
    dict.insert("provider_email".into(), opt(&branding.provider_email));
    dict.insert("provider_phone".into(), opt(&branding.provider_phone));

    dict.insert("bank_name".into(), opt(&branding.bank_name));
    dict.insert("bank_iban".into(), opt(&branding.bank_iban));
    dict.insert("bank_bic".into(), opt(&branding.bank_bic));
    dict.insert("bank_routing".into(), opt(&branding.bank_routing));
    dict.insert("bank_account".into(), opt(&branding.bank_account));
    dict.insert("payment_terms".into(), opt(&branding.invoice_payment_terms));

    let line_values: Vec<Value> = lines
        .iter()
        .map(|l| {
            let mut ld = Dict::new();
            ld.insert("description".into(), l.description.clone().into_value());
            ld.insert("minutes".into(), (l.minutes as i64).into_value());
            ld.insert("rate_cents".into(), l.rate_cents.into_value());
            ld.insert("amount_cents".into(), l.amount_cents.into_value());
            Value::Dict(ld)
        })
        .collect();
    dict.insert("lines".into(), Value::Array(line_values.as_slice().into()));

    dict
}

/// Render an invoice to a PDF byte vector.
///
/// Deterministic: identical inputs produce byte-identical output.
#[allow(clippy::too_many_arguments)]
pub fn render_invoice_pdf(
    invoice: &Invoice,
    lines: &[InvoiceLine],
    client_name: &str,
    client_address: Option<&str>,
    client_tax_id: Option<&str>,
    branding: &OrgBranding,
) -> anyhow::Result<Vec<u8>> {
    let engine = build_engine();
    let inputs = build_inputs(
        invoice,
        lines,
        client_name,
        client_address,
        client_tax_id,
        branding,
    );

    let doc: typst_layout::PagedDocument = engine
        .compile_with_input(inputs)
        .output
        .map_err(|e| anyhow::anyhow!("Typst compilation failed: {e}"))?;

    let options = typst_pdf::PdfOptions {
        timestamp: None,
        ..Default::default()
    };

    let pdf_bytes =
        typst_pdf::pdf(&doc, &options).map_err(|e| anyhow::anyhow!("PDF export failed: {e:?}"))?;

    Ok(pdf_bytes)
}
