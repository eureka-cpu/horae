# Horae Design Language

Horae follows the Invoicer design system: editorial warm-dark surfaces, serif display type, humanist sans-serif for UI, and monospace numerals — an accountancy aesthetic that suits time-tracking and invoicing. Visual prototypes with pixel-accurate examples live in `design/project/dark.dc.html`.

## Color Palette

All colors are CSS custom properties in `assets/css/horae.css`.

### Surface layers (darkest → lightest)

| Token | Value | Usage |
|---|---|---|
| `--color-bg` | `#100F0C` | Page background (void) |
| `--color-bg-secondary` | `#1A1813` | Cards, nav bar, sidebar |
| `--color-bg-tertiary` | `#232019` | Table headers, hover states |
| `--color-bg-overlay` | `#2E2A22` | Dropdowns, tooltips |

### Text

| Token | Value | Usage |
|---|---|---|
| `--color-text` | `#EFEAE0` | Primary body text |
| `--color-text-secondary` | `#A29C8D` | Labels, captions, sidebar links |
| `--color-text-muted` | `#7C7565` | Placeholders, section headers |

### Borders

| Token | Value | Usage |
|---|---|---|
| `--color-border` | `#322E26` | Card borders, nav border, table edges |
| `--color-border-light` | `#262219` | Table row dividers |

### Accent

| Token | Value | Usage |
|---|---|---|
| `--color-primary` | `#4FB79A` | Pine-300 — logo, active links, timer, focus ring |
| `--color-primary-dark` | `#3D9E84` | Button hover |
| `--color-primary-light` | `#6ECAB0` | Active sidebar link text |
| `--color-primary-bg` | `rgba(79,183,154,0.14)` | Active sidebar link background |
| `--color-accent` | `#D99A3C` | Brass — invoicing context, send/bill actions |
| `--color-accent-bg` | `rgba(217,154,60,0.14)` | Brass tint background |
| `--color-pine` | `#1F5C4D` | Deep pine — solid fills, marks |

### Semantic (warm dark-tuned)

| Token | Value |
|---|---|
| `--color-success` | `#3FB489` |
| `--color-warning` | `#D6A24A` |
| `--color-danger` | `#E06661` |
| `--color-info` | `#4FB79A` (reuses pine-300) |

Semantic backgrounds use solid dark tints (`--color-success-bg: #15291F`, etc.) rather than rgba.

## Typography

Three typefaces loaded from Google Fonts:

```
@import url('https://fonts.googleapis.com/css2?family=Newsreader:ital,opsz,wght@0,6..72,400;0,6..72,500;0,6..72,600;1,6..72,400&family=Instrument+Sans:wght@400;500;600;700&family=IBM+Plex+Mono:wght@400;500;600&display=swap');
```

| Role | Family | Token | Usage |
|---|---|---|---|
| Display / headings | Newsreader (serif) | `--font-family-display` | `h1–h6`, brand wordmark, card titles, page titles, auth logo |
| UI / body | Instrument Sans (humanist sans) | `--font-family` | Body, labels, buttons, nav links, form inputs |
| Numerals | IBM Plex Mono | `--font-family-mono` | Timer, stat values, amounts, hours, rates, dates |

## Border Radius

| Token | Value | Usage |
|---|---|---|
| `--radius-sm` | `4px` | Micro elements |
| `--radius` | `6px` | Controls: inputs, selects |
| `--radius-btn` | `8px` | Buttons |
| `--radius-lg` | `11px` | Cards, table containers |
| `--radius-panel` | `20px` | Auth card, large panels |
| `--radius-full` | `9999px` | Badge pills |

## Layout

- **Sidebar** (`--sidebar-width: 264px`): full-height left rail on the warm dark surface (`--color-bg-secondary`), 1px right border, sticky. Holds the brand, a start-timer action, grouped navigation, and the signed-in user; there is no separate top nav. Collapses to a 68px icon strip.
- **Content area**: scrollable, generous padding, `--color-bg` (`#100F0C`) background.

## Component Inventory

### Sidebar (`src/components/sidebar.rs`)

Full-height left rail. The brand row leads with the Horae mark and wordmark plus the golden accent dot. Nav rows default to `--color-text-secondary`; hover darkens the background to `--color-bg-tertiary`; the active row raises to `--color-bg-tertiary` with a `--color-border` hairline and swaps its glyph for a leading pine dot. Section labels: all-caps, wide letter-spacing, muted warm gray.

### Timer Widget (`src/components/timer_widget.rs`)

`HH:MM:SS` in IBM Plex Mono (`--font-family-mono`), pine-300 when stopped, green (`--color-success`) when running.

### Data Tables (`src/components/table.rs`)

Wrapped in `.table-container` (border + 11px radius). Headers: uppercase, 0.06em letter-spacing, tertiary warm surface. Row hover on `--color-bg-tertiary`. Last row has no bottom border.

### Forms (`src/components/form.rs`)

Inputs on `--color-bg` (void — darkest) with warm border `#3d382e`. Focus ring: `--color-primary` border + 18% pine glow. Instrument Sans for all form text.

### Status Badges (`src/components/badge.rs`)

Pill shape (9999px radius). Each semantic badge includes a 7px colored dot (via `::before`) + tinted background + warm border. Neutral badge uses tertiary surface + border.

### Buttons (`src/components/`)

- **Primary**: pine-300 fill, dark green text `#0d211b`
- **Secondary**: tertiary surface, warm border
- **Accent**: brass fill `#D99A3C`, dark text — use for "Send invoice" and billing actions
- **Danger**: transparent with dark red border `#6e3634`, becomes tinted on hover
- **Ghost**: transparent, warm border

## Component Conventions

- One component per file in `src/components/`
- Props use `snake_case`
- Components are `#[component]` functions returning `Element`
- State uses `use_signal` (local) or `use_resource` (async server data)
- No global mutable state — pass data down via props or context

## Tokens & utilities

Everything is driven by CSS custom properties in `:root` (`assets/css/horae.css`) —
never hardcode a colour, spacing, or radius in a rule; reference the token so the
palette stays themeable and consistent. Notable token groups:

- Colour: `--color-{bg,bg-secondary,bg-tertiary,bg-overlay}`, `--color-{text,text-secondary,text-muted}`, `--color-primary*`, `--color-accent*`, and semantic `--color-{success,warning,danger,info}` with `-bg` / `-fg` / `-line` tints.
- Foreground-on-fill: `--color-on-{primary,accent,pine}` (text over a solid control).
- Chrome: `--color-border`, `--color-border-input`, `--color-border-danger`, `--ring` / `--ring-soft` (focus glows).
- Scale: `--space-1..16`, `--font-size-*`, `--radius-*`.

On top of the tokens is a **Tailwind-style utility layer** — `flex`,
`items-center`, `justify-between`, `gap-4`, `p-4`, `text-sm`, `font-semibold`,
`text-secondary`, `bg-secondary`, `rounded-lg`, … plus responsive variants
(`md:flex-row`, `lg:grid-cols-3`). The numeric spacing scale matches Tailwind's
(`p-4` = `--space-4` = 1rem). Compose utilities in markup for layout/spacing; reach
for a semantic component class (`.btn`, `.card`, `.badge`, `.nav-item`) for
anything reused.

This layer is **generated** from the design scale by the `cssgen` crate (the
Node-free equivalent of a Tailwind build) into `assets/css/horae-utils.css`, which
`app.rs` loads alongside `horae.css`. It is plain committed CSS at runtime — no
build step in the app. After changing the scale in `crates/cssgen/src/main.rs`:

```sh
cargo run -p cssgen        # regenerate and commit horae-utils.css
cargo run -p cssgen -- --check   # CI parity: fails if the committed file is stale
```

`nix flake check` runs the `--check` so drift can't merge. Tokens and semantic
component classes stay hand-written in `horae.css`; the generator owns only the
mechanical utility + responsive matrix.

## Pages

| Route | Component | Description |
|---|---|---|
| `/auth/login` | `Login` | Email + password form |
| `/auth/register` | `Register` | Registration (admin only after first user) |
| `/` | `Dashboard` | Stats overview + active timer |
| `/clients` | `ClientList` | Table of clients |
| `/clients/:id` | `ClientDetail` | Projects under client |
| `/projects` | `ProjectList` | All projects |
| `/projects/:id` | `ProjectDetail` | Project tasks + time entries |
| `/time` | `TimeList` | Time entry list with filters |
| `/invoices` | `InvoiceList` | Invoice table |
| `/invoices/:id` | `InvoiceDetail` | Invoice line items |
| `/admin/users` | `AdminUsers` | User management |
| `/settings` | `Settings` | App + plugin settings |

## Accessibility

- Keyboard navigation throughout
- ARIA labels on icon-only buttons
- Color contrast meets WCAG AA — `#EFEAE0` on `#1A1813` is ~9:1
- Form inputs have associated `<label>` elements
- Status indicated by text + color (never color alone) — badges include both dot and text label

## Interaction Principles

- Timer state is reactive: the running timer increments via `use_interval`
- All data mutations go through `#[server]` functions — never direct fetch calls
- Optimistic UI where appropriate; rollback on error
- Loading states shown inline, not full-page spinners
