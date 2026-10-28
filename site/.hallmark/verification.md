# Public Pages content and navigation verification

2026-10-26. Scope: home, roadmap, playful dossier, shared chrome and styled 404.
Implements the owner's content/IA brief supplied on task `t_c5d4948d`.
No product code or dependency changes. Lead review, integration and publication are
recorded below; product and qualification limits remain unchanged.

## Content and material

- Preserved Epilogue / IBM Plex Mono, near-black, lilac and cyan visual system.
- Replaced the turntable image with the existing synthetic-library screenshot from
  `prototypes/oxidj-library-ui/evidence/selected-track-details.png`, captioned Prototype.
- One shared dated source (`src/data/roadmap.ts`) supplies current M4 focus and status.
- One Status per content page distinguishes current local catalog/import/viewer from
  production GUI, accessibility/platform, full vendor compatibility and 500k/60fps work.
- Shared Home / Roadmap / Updates / Source header and footer; dossier is footer-only.
- Home explains vendor lock-in, XML versus OneLibrary and the reference-only local slice.
- Try it covers Linux exercised, macOS/Windows unverified, prerequisites and init/import/list.
- Roadmap has compact M0-M4, stable m0-m7 anchors, short Later group and dated changelog.
- Metadata, canonical links, local licensed fonts, 1200x630 hero-derived social image,
  robots.txt, sitemap.xml and project-base-safe 404 are emitted without client rendering.
- No private audio or personal library material was added. Font licences accompany fonts.

## Word counts

Whitespace-delimited visible text in built HTML, including shared navigation/footer;
head, scripts, styles and comments excluded. Same `scripts/word-count.mjs` for both.
Baseline is an actual build of commit `6c409c9`, archived into ignored local evidence.

| Route   | Before | After |
| ------- | -----: | ----: |
| Home    |    332 |   432 |
| Roadmap |    804 |   378 |

Home grows because the owner requested an executable try-it guide. No numeric limit
was specified. Roadmap duplication is removed.

## Executed verification

- `bun run typecheck`, `bun run lint`, `bun run test`, `bun run format:check`,
  `bun run build`: exit 0. Vitest: 6 files, 47 tests.
- Build verifier: 4 entries, 70 local references under `/oxidj/`, all eight milestone
  anchors, metadata, static copy, CSS font references and 1200x630 image checked.
- `node scripts/verify-links.mjs`: exit 0; 10 distinct external GitHub links return 200;
  referenced Markdown heading anchors match their source files.
- `node scripts/verify-cli.mjs`: exit 0; actual init, empty list, import of lawful generated
  WAV, saved list all exit 0. Saved track ID/path/digest/size and unchanged original checked.
- `cargo xtask ci`: exit 0:
  `=> ci summary: layers=ok fmt=ok clippy=ok check=ok  [all pass]`.
- `git diff --cached --check -- . ':!site/public/fonts/*'`: exit 0. Upstream font
  licence files retain their original whitespace/CRLF formatting.

## Browser checks

Production preview uses `/oxidj/` on port 4183. External existing Puppeteer/Chromium;
no site runtime or development dependency was added.

`HALLMARK_PUPPETEER=/home/senci/dev/oxidj/.hallmark-tooling/node_modules/puppeteer-core
CHROME_PATH=/usr/sbin/chromium node scripts/verify-browser.mjs`: exit 0.

All four entries at **390px and 1440px**:

- No page, console, resource or HTTP errors; no horizontal overflow or broken images.
- Loaded local Epilogue and IBM Plex Mono; zero third-party resource requests.
- All link/button boxes meet 44px width and height.
- Small-text computed contrast minimum **7.90:1** (404: **8.10:1**), above AA 4.5:1.
- Tab finds skip link; Enter reaches main; essential content survives JavaScript off.
- Dossier asylum button still produces the expected joke.

Captures and JSON: ignored `.hallmark/final/`. Visually inspected home at both sizes,
roadmap desktop and dossier phone; no clipped content or substantive layout defect.

## Hallmark

Ran actual upstream `sloplint.mjs` from the existing external bundle at commit
`7ec868225c6caa040558c321ee0fefa0e73f83a8`, with `src/styles/public.css`, atmospheric
render mode, and development URLs `/`, `/roadmap.html`, `/dossier.html` on port 5176.
Development URLs exercise Vite's actual static HTML fragments and retain CSS stamps.
Linting untransformed HTML directly initially reported missing public-folder image/fonts
and metadata; that file-only render is not the application's served representation.
Production output was independently browser-tested above.

Final sweep: **exit 0, 0 FAIL, 0 open REFLEX, 2 WARN, 8 WAIVED, 4 NOTE**.

Warnings judged:

- Side stripe on Status: intentionally retained as the single shared informational callout.
- Numeric h2 selector: false positive; Status heading contains no digits.

Recorded design waivers retain compact documentation sections (59/77), compact Banner
hero (79), requested black redaction (7b), and always-enabled joke button (26b).
Screen-word-count notes are expected in a CLI guide/roadmap, not invented marketing mass.
The owner specifically requires straight quotes, overriding the typographic-quote hint.

## Integration and deployment

Lead review passed. Commit `ff79e20eb74e6020dc3cab5189cabd0e8c1a2fc2` is on
`master` and `origin/master`. GitHub Pages workflow run
[40386661061](https://github.com/Senci/oxidj/actions/runs/40386661061) completed
success for that exact commit.

Read-back on 2026-10-26 returned HTTP 200 for home, roadmap, dossier and styled
404 at `https://senci.github.io/oxidj/`. The project-level `robots.txt` ships as
requested; crawler policy is authoritative only at origin root on GitHub Pages.
No new photography or other material is needed.
