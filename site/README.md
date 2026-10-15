# FILE #0042 — OxiDJ teaser

A single-page, static teaser site for **OxiDJ**, an open-source, Rust-first DJ
library manager. The page presents itself as a classified dossier about the
visitor's (fictional) DJ library — dark, dry, and ominous — with one dark
twist: it behaves as if it is watching the visitor and judging their taste.

## What it is

`FILE #0042` is a dark, funny intelligence dossier about _your_ DJ library.
A "LIVE ANALYSIS" panel reacts to your mouse, typing, and clicks with deadpan
observations, and the "REQUEST ASYLUM" button cycles through a series of
curt denials. It is honest that the project is early (M0: research and
architecture) while being entertaining.

## Quick start

All commands run locally and offline — no network, no CI, no external
services at build or test time. The project uses **Bun** for dependencies and
scripts (per the project's package-manager preference).

```bash
# install dependencies (one-time)
bun install

# run the unit tests
bun run test

# type-check + lint
bun run typecheck   # tsc --noEmit (strict)
bun run lint       # eslint

# format
bun run format        # prettier --write .
bun run format:check  # prettier --check .

# build the static site into site/dist
bun run build

# preview the production build locally
bun run preview
```

> If you prefer npm, the same scripts work via `npm run test`, `npm run
build`, etc. Dependencies were installed with Bun.

## Concept

The page is a classified case file ("FILE #0042") about the visitor's DJ
library. It pairs a fake dossier and a deadpan evidence log with a "dark
twist": a **LIVE ANALYSIS** panel that reacts to the visitor's behaviour
(mouse movement, typing, clicks) as though it is watching and judging them,
plus a **REQUEST ASYLUM** button that always says no.

## Project structure

```
site/
├── index.html                 # semantic page shell (header, dossier, evidence log, live analysis, asylum, footer)
├── package.json
├── tsconfig.json              # strict: strict, noUncheckedIndexedAccess, noImplicitOverride, exactOptionalPropertyTypes
├── eslint.config.mjs          # typescript-eslint strict + prettier
├── prettier.config.mjs
├── vite.config.ts             # base './' so it works on GitHub Pages subpaths
├── vitest.config.ts
├── src/
│   ├── main.ts                # entry: hydrates the page
│   ├── vite-env.d.ts
│   ├── logic/                 # pure, unit-tested teaser logic (no DOM)
│   │   ├── redaction.ts       #   redact() — redacted bars helper
│   │   ├── asylum.ts          #   getAsylumDenial() — denial cycler
│   │   ├── analysis.ts        #   getLiveObservation() — deadpan observations
│   │   └── evidenceLog.ts     #   generateEvidenceLog() + format + guard
│   ├── ui/
│   │   └── app.ts             # DOM wiring (render + event listeners)
│   ├── styles/
│   │   └── style.css          # dark/light theme, scanlines, stamp, reduced-motion
│   └── __tests__/             # vitest unit tests for the logic
│       ├── redaction.test.ts
│       ├── asylum.test.ts
│       ├── analysis.test.ts
│       └── evidenceLog.test.ts
└── dist/                      # build output (generated)
```

The teaser logic lives in `src/logic/` as pure functions so it can be unit
tested without a DOM; `src/ui/app.ts` wires that logic to the page.

## Accessibility & contrast

- **Semantic HTML**: `<header>`, `<section>` with `aria-labelledby`, `<dl>`
  for the dossier, `<ol>` for the evidence log, a real `<button>` for asylum,
  and a `<footer>`.
- **Live regions**: the LIVE ANALYSIS and asylum outputs use
  `aria-live="polite"` so screen readers announce updates.
- **Reduced motion**: all transitions and animations are disabled under
  `prefers-reduced-motion: reduce`.
- **Colour is not the only signal**: the status line and live-analysis output
  are paired with the red accent but also carry distinct, descriptive text.

### Contrast (WCAG AA)

Dark is the default theme. The page flips to a light "reading room" palette
when the OS requests `prefers-color-scheme: light` (via CSS custom
properties). Every text pairing below clears **WCAG AA (4.5:1)** in both
themes:

| Element                | Dark                   | Ratio   | Light                  | Ratio   |
| ---------------------- | ---------------------- | ------- | ---------------------- | ------- |
| Body text              | `#e8e6e3` on `#0a0a0f` | 15.86:1 | `#1a1a14` on `#f4f2ea` | 15.59:1 |
| Body text (on panel)   | `#e8e6e3` on `#111118` | 15.09:1 | `#1a1a14` on `#eceade` | 14.47:1 |
| Muted / secondary text | `#9a978f` on `#0a0a0f` | 6.77:1  | `#5c584c` on `#f4f2ea` | 6.34:1  |
| Status / accent-bright | `#e74c3c` on `#0a0a0f` | 5.17:1  | `#c0392b` on `#f4f2ea` | 4.85:1  |
| Stamp (amber)          | `#c9a36a` on `#0a0a0f` | 8.40:1  | `#7a5c2e` on `#f4f2ea` | 5.51:1  |
| Button text            | `#ffffff` on `#c0392b` | 5.44:1  | `#ffffff` on `#a93226` | 6.62:1  |
| Button hover text      | `#ffffff` on `#96281b` | 8.03:1  | `#ffffff` on `#8c2a1f` | 8.52:1  |
| Link (footer)          | `#e74c3c` on `#0a0a0f` | 5.17:1  | `#c0392b` on `#f4f2ea` | 4.85:1  |

## Licence

MIT. See [LICENSE](./LICENSE). No GPL-derived content or dependencies.
