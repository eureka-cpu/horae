// Horae invoice template — receives data via sys.inputs.
// Deterministic: no auto-timestamp, no nondeterministic ordering.

#import sys: inputs

#set document(date: none, title: inputs.invoice_number)
#set page(paper: "a4", margin: (top: 2cm, bottom: 2cm, left: 2cm, right: 2cm))
#set text(size: 10pt)

// ── Helpers ──────────────────────────────────────────────────────────────────

#let fmt-money(cents, currency) = {
  let abs = calc.abs(cents)
  let whole = calc.floor(abs / 100)
  let frac = calc.rem(abs, 100)
  let sign = if cents < 0 { "−" } else { "" }
  let frac-str = if frac < 10 { "0" + str(frac) } else { str(frac) }
  sign + currency + " " + str(whole) + "." + frac-str
}

#let fmt-hours(minutes) = {
  let h = calc.floor(minutes / 60)
  let m = calc.rem(minutes, 60)
  let m-str = if m < 10 { "0" + str(m) } else { str(m) }
  str(h) + ":" + m-str
}

// ── Header ───────────────────────────────────────────────────────────────────

#grid(
  columns: (1fr, 1fr),
  align: (left, right),
  [
    #if inputs.provider_name != none [
      #text(size: 14pt, weight: "bold", inputs.provider_name) \
    ]
    #if inputs.provider_address != none [
      #text(size: 9pt, fill: luma(100), inputs.provider_address) \
    ]
    #if inputs.provider_tax_id != none [
      #text(size: 9pt, fill: luma(100), "Tax ID: " + inputs.provider_tax_id) \
    ]
    #if inputs.provider_email != none [
      #text(size: 9pt, fill: luma(100), inputs.provider_email) \
    ]
    #if inputs.provider_phone != none [
      #text(size: 9pt, fill: luma(100), inputs.provider_phone) \
    ]
  ],
  [
    #text(size: 20pt, weight: "bold", fill: rgb("#4FB79A"), "INVOICE") \
    #v(4pt)
    #text(size: 11pt, weight: "semibold", inputs.invoice_number) \
    #v(8pt)
    #text(size: 9pt, fill: luma(100), "Issued: " + inputs.issued_on) \
    #text(size: 9pt, fill: luma(100), "Due: " + inputs.due_on)
  ],
)

#v(24pt)

// ── Bill To ──────────────────────────────────────────────────────────────────

#block(
  width: 50%,
  [
    #text(size: 9pt, weight: "semibold", fill: luma(100), "BILL TO") \
    #v(4pt)
    #text(weight: "semibold", inputs.client_name) \
    #if inputs.client_address != none [
      #text(size: 9pt, inputs.client_address) \
    ]
    #if inputs.client_tax_id != none [
      #text(size: 9pt, "Tax ID: " + inputs.client_tax_id) \
    ]
  ],
)

#v(24pt)

// ── Line items ───────────────────────────────────────────────────────────────

#let currency = inputs.currency

#table(
  columns: (1fr, auto, auto, auto),
  stroke: none,
  inset: (x: 8pt, y: 6pt),

  // Header
  table.hline(stroke: 0.8pt + luma(180)),
  table.header(
    text(size: 9pt, weight: "semibold", "Description"),
    text(size: 9pt, weight: "semibold", "Hours"),
    text(size: 9pt, weight: "semibold", "Rate"),
    align(right, text(size: 9pt, weight: "semibold", "Amount")),
  ),
  table.hline(stroke: 0.5pt + luma(210)),

  // Rows
  ..for line in inputs.lines {
    (
      text(size: 9pt, line.description),
      text(size: 9pt, fmt-hours(line.minutes)),
      text(size: 9pt, fmt-money(line.rate_cents, currency) + "/hr"),
      align(right, text(size: 9pt, fmt-money(line.amount_cents, currency))),
    )
  },

  // Total
  table.hline(stroke: 0.8pt + luma(180)),
  table.cell(colspan: 3, align(right, text(weight: "bold", "Total"))),
  align(right, text(weight: "bold", fmt-money(inputs.total_cents, currency))),
)

#v(32pt)

// ── Payment details ──────────────────────────────────────────────────────────

#if inputs.bank_name != none or inputs.bank_iban != none or inputs.bank_routing != none [
  #text(size: 9pt, weight: "semibold", fill: luma(100), "PAYMENT DETAILS") \
  #v(4pt)
  #if inputs.bank_name != none [
    #text(size: 9pt, "Bank: " + inputs.bank_name) \
  ]
  #if inputs.bank_iban != none [
    #text(size: 9pt, "IBAN: " + inputs.bank_iban) \
  ]
  #if inputs.bank_bic != none [
    #text(size: 9pt, "BIC: " + inputs.bank_bic) \
  ]
  #if inputs.bank_routing != none [
    #text(size: 9pt, "Routing: " + inputs.bank_routing) \
  ]
  #if inputs.bank_account != none [
    #text(size: 9pt, "Account: " + inputs.bank_account) \
  ]
  #v(16pt)
]

// ── Terms & notes ────────────────────────────────────────────────────────────

#if inputs.payment_terms != none [
  #text(size: 9pt, weight: "semibold", fill: luma(100), "TERMS") \
  #v(4pt)
  #text(size: 9pt, inputs.payment_terms) \
  #v(16pt)
]

#if inputs.notes != none [
  #text(size: 9pt, weight: "semibold", fill: luma(100), "NOTES") \
  #v(4pt)
  #text(size: 9pt, inputs.notes) \
]
