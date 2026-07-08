# Horae Design Language

Horae follows a GitHub/Gitea dark-mode visual language: dark surfaces, subtle borders, teal accent. Clean and functional — no JS frameworks, all interactivity via Dioxus reactivity.

## Color Palette

All colors are CSS custom properties in `assets/css/horae.css`.

### Surface layers (darkest → lightest)

| Token | Value | Usage |
|---|---|---|
| `--color-bg` | `#0d1117` | Page background |
| `--color-bg-secondary` | `#161b22` | Cards, nav bar, sidebar |
| `--color-bg-tertiary` | `#21262d` | Table headers, form inputs, hover |
| `--color-bg-overlay` | `#30363d` | Dropdowns, tooltips |

### Text

| Token | Value | Usage |
|---|---|---|
| `--color-text` | `#e6edf3` | Primary body text |
| `--color-text-secondary` | `#8b949e` | Labels, captions, sidebar links |
| `--color-text-muted` | `#6e7681` | Placeholders, section headers |

### Borders

| Token | Value | Usage |
|---|---|---|
| `--color-border` | `#30363d` | Card borders, nav border, table edges |
| `--color-border-light` | `#21262d` | Table row dividers |

### Accent

| Token | Value | Usage |
|---|---|---|
| `--color-primary` | `#4aa398` | Logo, active links, timer, focus ring |
| `--color-primary-dark` | `#3d8c82` | Button hover |
| `--color-primary-light` | `#5ebdb2` | Active sidebar link text |
| `--color-primary-bg` | `rgba(74,163,152,0.12)` | Active sidebar link background |

### Semantic (dark-tuned)

| Token | Value |
|---|---|
| `--color-success` | `#3fb950` |
| `--color-warning` | `#d29922` |
| `--color-danger` | `#f85149` |
| `--color-info` | `#388bfd` |

Semantic backgrounds are 15% opacity tints of the base color.

## Typography

System font stack only — no web fonts:

```
-apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans", Helvetica, Arial, sans-serif
```

Monospace for timer displays:
```
"SFMono-Regular", Consolas, "Liberation Mono", Menlo, Courier, monospace
```

## Layout

- **Top nav** (`--nav-height: 56px`): dark `#161b22` background with teal brand name, 1px bottom border. Not a colored bar.
- **Sidebar** (`--sidebar-width: 240px`): same dark surface as nav, fixed position.
- **Content area**: scrollable, `max-width: 1200px`, generous padding, `--color-bg` (#0d1117) background.

## Component Inventory

### Navigation (`src/components/nav.rs`)
Dark bar (`--color-bg-secondary`), teal brand text, secondary nav links that lighten on hover.

### Sidebar (`src/components/sidebar.rs`)
Fixed left sidebar. Links default to `--color-text-secondary`; hover darkens background to `--color-bg-tertiary`; active uses teal tint background + `--color-primary-light` text.

### Timer Widget (`src/components/timer_widget.rs`)
`HH:MM:SS` in monospace, teal when stopped, green (`--color-success`) when running.

### Data Tables (`src/components/table.rs`)
Wrapped in `.table-container` (adds border + border-radius). Headers on `--color-bg-tertiary`. Row hover on `--color-bg-tertiary`. Last row has no bottom border.

### Forms (`src/components/form.rs`)
Inputs on `--color-bg-tertiary` with `--color-border` border. Focus ring: `--color-primary` border + 20% teal glow.

### Status Badges (`src/components/badge.rs`)
Pill shape. Color + background tint only — no border on semantic badges. Neutral badge keeps `--color-border`.

## Component Conventions

- One component per file in `src/components/`
- Props use `snake_case`
- Components are `#[component]` functions returning `Element`
- State uses `use_signal` (local) or `use_resource` (async server data)
- No global mutable state — pass data down via props or context

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
- Color contrast meets WCAG AA (4.5:1 for text) — `#e6edf3` on `#161b22` is ~11:1
- Form inputs have associated `<label>` elements
- Status indicated by text + color (never color alone)

## Interaction Principles

- Timer state is reactive: the running timer increments via `use_interval`
- All data mutations go through `#[server]` functions — never direct fetch calls
- Optimistic UI where appropriate; rollback on error
- Loading states shown inline, not full-page spinners
