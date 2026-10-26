# OxiDJ project site

The Pages root is a Hallmark-built teaser. The dated development timeline is preserved at
`./roadmap.html`, and the earlier FILE #0042 dossier remains reachable at `./dossier.html`.
All three entries build to `dist/`; Vite's relative base supports `/oxidj/` and local previews.

## Keep it current

`src/data/roadmap.ts` owns the dated milestone snapshot used by the timeline. Update it when
reviewed project status changes, and keep the teaser's short progress summary and snapshot
date consistent. Canonical decisions remain in `idea.md`, `plan/MVP.md`, `plan/STATUS.md`
and ADRs; the site is not a second decision record.

Snapshot: 2026-10-26. Bounded local M1, M2 and provisional M3 acceptance is complete and
independently reviewed. See `plan/MVP.md` and `plan/STATUS.md` for evidence. Production GUI
admission, accessibility/platform checks, vendor interoperability, and 500k/60fps qualification
remain separate and unpassed.

## Local checks

```sh
bun install --frozen-lockfile
bun run dev
bun run typecheck
bun run lint
bun run test
bun run format:check
bun run build
bun run preview
```

`.github/workflows/pages.yml` runs the site checks, builds `site/dist`, uploads that directory
and deploys with the GitHub Pages environment. The repository must select **GitHub Actions**
as its Pages source. A local build does not confirm live deployment; inspect the Actions run
and read back the published URL after pushing. The latest recorded deployment attempt failed
because Pages was not enabled/configured for Actions.

## Design and verification

Teaser only: Ink theme, editorial genre, Conversational FAQ with a Stage hero. The CLI help
excerpt was captured from the actual repository; subcommand arguments are explicitly omitted.
No GUI screenshot is republished because the available captures contain local paths. No
stand-in imagery, fabricated UI, stock photos or missing-material slots are used.

- `src/styles/teaser.css`: stamp, critique, layout and review record
- `src/styles/tokens.css` and `base.css`: Hallmark scaffold, cool-tinted neutral adjustment
- `.hallmark/preflight.json` and `log.json`: material and design decisions
- `.hallmark/verification.md`: commands, outcomes, judged gates and capture locations

The installed Hallmark bundle lacked its executable tools and theme files. Verification used
upstream `https://github.com/Nutlope/hallmark` at
`7ec868225c6caa040558c321ee0fefa0e73f83a8`, checked out locally in the uncommitted
`../.hallmark-tooling`. Its scripts require Node, puppeteer-core and Chromium for rendering.
Do not add that checkout or its dependencies to this project.

The roadmap and dossier retain their existing designs; Hallmark sweeps target the new teaser,
not a silent redesign of those routes. The teaser remains fully readable with JavaScript off.
The roadmap keeps its original TypeScript renderer and provides a no-script source-plan link.

## Licence

Site: MIT, see `LICENSE`. Fonts load from Google Fonts: Epilogue and IBM Plex Mono.
