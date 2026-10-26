# OxiDJ project site

The Pages teaser lives at `./`, the evidence-linked timeline at `./roadmap.html`, and the
preserved earlier dossier at `./dossier.html`. All three entries build to `dist/`; Vite's
relative base supports `/oxidj/` and local previews.

## Keep it current

`src/data/roadmap.ts` owns the dated milestone snapshot. Vite renders it into the roadmap
HTML at development/build time through `src/logic/roadmapMarkup.ts`; no client JavaScript
is required to read milestones, updates, goals or qualification caveats. Tests verify every
source item and evidence link survives rendering. Keep the teaser summary and snapshot
consistent. Canonical decisions remain in `idea.md`, `plan/MVP.md`, `plan/STATUS.md` and ADRs.

Snapshot: 2026-10-26. Bounded local M1, M2 and provisional M3 acceptance is complete and
independently reviewed. Production GUI admission, accessibility/platform checks, vendor
interoperability, and 500k/60fps qualification remain separate and unpassed. The dated
snapshot is not a blanket statement about every subsequent interoperability experiment.

## Local checks

```sh
bun install --frozen-lockfile
bun run dev
bun run typecheck
bun run lint
bun run test
bun run format:check
bun run build
bun run preview --host 127.0.0.1 --port 4174 --base /oxidj/
```

The build verifier checks all three entries, local assets, project-relative links and HTML
anchors under a synthetic `/oxidj/` base. It also checks static roadmap content and the
script-free teaser. `.github/workflows/pages.yml` runs the site checks and deploys `site/dist`
on relevant master pushes. Pages uses **GitHub Actions** as its source. Production:
<https://senci.github.io/oxidj/>. Check the Actions deployment associated with the merged
commit before claiming a new version is live.

## Design and verification

Both active pages use the owner-requested Tokyo-night black, purple primary and cyan
secondary. `src/styles/night.css` owns their shared tokens and foundations; `teaser.css`
and `roadmap.css` own their compositions. The archived dossier is unchanged.

- The teaser shows the actual reference-only library flow, with a readable semantic
  mobile alternative; actual CLI help remains below the hero.
- The roadmap retains the milestone spine, typed evidence, source links and qualification
  boundaries, with larger body/details text and corrected static completion summaries.
- A small locally hosted, cropped Pixabay vinyl photograph appears below each hero.
  It is atmosphere, not an OxiDJ playback screenshot or an endorsement. The prominent
  cartridge trademark in the original is excluded from the crop. See `public/images/README.md`.
- CSS scroll-linked photo parallax progressively enhances supported desktop browsers.
  It is static on mobile, unsupported browsers and with reduced motion; content never
  depends on animation or a reveal script.

Design decisions: `design.md`, `.hallmark/preflight.json`, `.hallmark/log.json`.
Check evidence: `.hallmark/verification.md`. Generated captures and local review tooling
are not deliverable dependencies. The installed Hallmark bundle lacks executable tools;
render verification uses upstream <https://github.com/Nutlope/hallmark> at
`7ec868225c6caa040558c321ee0fefa0e73f83a8`, with external puppeteer-core and Chromium.

## Licence

Site code: MIT, see `LICENSE`. The Pixabay photograph is **not MIT** and is separately
licensed under the Pixabay Content License, documented in `public/images/README.md` and
credited on both pages. Fonts load from Google Fonts: Epilogue and IBM Plex Mono, with
local fallback faces.
