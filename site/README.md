# OxiDJ public project site

Public routes: `./`, `./roadmap.html`, `./dossier.html`. `404.html` is also built.
Vite's relative base supports the production repository prefix `/oxidj/`.

## Content ownership

`src/data/roadmap.ts` owns current focus, snapshot date, working/unfinished status,
compact milestones and recent changes. Vite renders `site:*` and `roadmap:*` fragments
at development/build time through `src/logic/siteMarkup.ts` and `roadmapMarkup.ts`.
Shared header/footer, one Status per content page and SEO metadata work without scripts.
The home reference diagram and roadmap can each offer an explicitly activated, finite
trace when motion is allowed; their static labels and native disclosures remain usable
without JavaScript. The dossier joke and those optional traces are the only client-side
enhancements. Canonical product decisions stay in `idea.md`, `plan/MVP.md`,
`plan/STATUS.md` and the interoperability research.

## Checks

```sh
bun install --frozen-lockfile
bun run dev --host 127.0.0.1 --port 5176 --strictPort
bun run typecheck
bun run lint
bun run test
bun run format:check
bun run build
bun run preview --host 127.0.0.1 --port 4183 --strictPort --base /oxidj/
node scripts/word-count.mjs
node scripts/verify-links.mjs
node scripts/verify-cli.mjs
```

The build verifier checks all four entries, project-relative assets (including CSS font
URLs), internal anchors, metadata, static copy and the social image dimensions. The CLI
check generates a small lawful WAV, executes init/import/list and verifies unchanged audio.
It needs the repository Rust toolchain. All generated files live in ignored `.hallmark/final/`.

Optional browser checks need an **external**, existing Puppeteer installation, not a site
dependency. Set `HALLMARK_PUPPETEER` to its package root and `CHROME_PATH` to Chromium:

```sh
SITE_URL=http://127.0.0.1:4183/oxidj/ node scripts/verify-browser.mjs
node scripts/generate-social.mjs
```

`verify-browser.mjs` imports Puppeteer's current `lib/puppeteer/puppeteer-core.js` entry.
It checks 390px and 1440px, resources, console, overflow, loaded fonts, 44px targets,
computed small-text contrast, skip links, script-free facts and the dossier button.
`generate-social.mjs` captures the home hero from the development server on port 5176.
Hallmark uses the existing external upstream bundle at commit
`7ec868225c6caa040558c321ee0fefa0e73f83a8`; source/development routes retain CSS stamps
that production minification removes. See `.hallmark/verification.md` for real results.

## Publication

`.github/workflows/pages.yml` checks and deploys `site/dist` after relevant master pushes.
Production: <https://senci.github.io/oxidj/>. Lead review/integration precedes publication.
Verify the exact Actions head and read every published route before claiming deployment.
The 404 uses `/oxidj/` as its base so resources/navigation work even at a nested missing URL.
The sitemap uses canonical production URLs. A project-level `robots.txt` is shipped as
requested; crawler policy is only authoritative at the origin root on GitHub Pages.

## Design and licences

All pages share the existing Epilogue/IBM Plex Mono near-black, lilac and cyan system.
Fonts are self-hosted; SIL Open Font License texts ship in `public/fonts/`. Public images
are the existing synthetic-library prototype and its hero-derived social image. The old
turntable asset and unused home stylesheet were removed. See `public/images/README.md`.
Site/project code: MIT. No new runtime dependencies or private library material.
