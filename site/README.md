# OxiDJ public project site

Public routes: `./`, `./roadmap.html`, `./dossier.html`. `404.html` is also built.
Vite's relative base supports the production repository prefix `/oxidj/`.

## Content ownership

`src/data/roadmap.ts` owns the snapshot date, focus, working/unfinished status,
milestones, recent changes and the screenshot list (size, alt, caption). Vite renders
`site:*` and `roadmap:*` fragments at build time through `src/logic/siteMarkup.ts`,
`roadmapMarkup.ts` and `waveArt.ts` (the hero's deterministic, synthetic 3-band SVG).
Every page works without scripts: facts, navigation, disclosures and images are static,
and each screenshot is a plain link to its full-resolution file. Canonical product
decisions stay in `idea.md`, `plan/MVP.md`, `plan/STATUS.md` and the research docs.

## Client enhancements (`src/ui/`)

- `lightbox.ts`: hand-written zoomable screenshot dialog (`role="dialog"`,
  `aria-modal`, focus trap and return, Esc, X button, click outside the image to close;
  wheel/pinch/double-click zoom anchored at the pointer, drag to pan, `+ - 0` and arrow
  keys). Pure math lives in `src/logic/zoom.ts` (scale 1 to 4, pan clamped to the edges).
- `app.ts`: scroll reveals via CSS scroll-driven animations (`view()`/`scroll()`),
  with an IntersectionObserver fallback that only hides content once scripts run; the
  hero glow follows the pointer; copy buttons on command blocks; the dossier joke.
- Motion is transform/opacity only. `prefers-reduced-motion: reduce` removes it all.

## Assets

- Screenshots: `scripts/crop-screenshots.py` (PIL via uv); provenance in
  `public/images/README.md`. Synthetic catalog only; inspect pixels before replacing.
- Fonts: `scripts/subset-fonts.py` subsets `fonts-src/*.ttf` to Latin WOFF2 (about
  11 KB each). The IBM Plex Mono subset is renamed OxiDJ Mono for its OFL Reserved Font
  Name. Licence texts ship in `public/fonts/`.

## Checks

```sh
bun install --frozen-lockfile
bun run typecheck && bun run lint && bun run format:check && bun run test
bun run build                         # vite build + scripts/verify-build.mjs
node scripts/word-count.mjs
node scripts/verify-links.mjs         # external GitHub links and anchors (network)
node scripts/verify-cli.mjs           # runs the documented CLI commands (Rust toolchain)
# rendered audit: serve dist/ at /oxidj/, then
uv run --no-project --with playwright python scripts/verify-browser.py http://127.0.0.1:4190/oxidj/ .hallmark/final
```

`verify-browser.py` drives the system Chromium (`CHROME_PATH`, default
`/usr/sbin/chromium`) at 390 and 1440 px on every route, with and without reduced
motion: console and request errors, horizontal overflow, WCAG contrast of visible text
on its blended background, 44 px mobile targets, layout shift, transferred bytes and an
end-to-end lightbox run. Screenshots land in the output directory (ignored).

## Publication

`.github/workflows/pages.yml` checks and deploys `site/dist` after relevant master
pushes. Production: <https://senci.github.io/oxidj/>. Nothing is deployed without a
push; verify the exact Actions head and read every route before claiming deployment.
The 404 uses `/oxidj/` as its base so it works at nested missing URLs.

Site/project code: MIT. No runtime dependencies (`happy-dom` is a test-only dev
dependency for the lightbox DOM tests).
