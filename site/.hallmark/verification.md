# Active Pages redesign verification

2026-10-26. Scope: teaser and roadmap only; archived dossier and product code unchanged.
Deployment is recorded on the task/PR after merge, not inferred from these local results.

## Before and preserved material

Before captures: `.hallmark/before/screen-{home,roadmap-html}-{desktop,phone}.png`.
The teaser was sparse/light; the roadmap had a useful spine, statuses and evidence links,
but tiny copy, decorative dots and stale M1/M0 static summaries.

Preserved: all three routes, project-relative asset base, teaser facts and CLI help excerpt,
all eight milestones, five updates, five product goals and every typed source/evidence link.
`src/data/roadmap.ts` is unchanged. The renderer moved from client DOM construction to
Vite HTML transformation so all roadmap content survives JavaScript being off.

Copy corrections/recompositions:

- “Working local-library code. Acceptance is still open.” became “Working local-library
  code. Production qualification remains open.” The bounded contract is complete.
- “Close the catalog acceptance gaps.” and M1 focus became “Local-library contract
  complete.” The M0-only verified summary became the M1/M2/provisional-M3 status.
- The roadmap hero introduction became “Shipped work, active development and later
  qualification, kept distinct.” Existing detail survives in the timeline and summary.
- Decorative field-note count, eyebrow labels, glued arrows, caps and dots were removed;
  the open-source/Rust/local principles remain in the teaser and linked source.
- M3's provisional status is explicit immediately below its completion heading. No
  production GUI, broad interoperability or 500k/60fps pass is implied.

The CLI help moved below the teaser hero, intact. Its historical actual-output provenance
is retained. The photo source/licence and crop are in `public/images/README.md`.

## Executed checks

All site commands exited 0 after implementation:

```sh
bun run typecheck
bun run lint
bun run test
bun run format:check
bun run build
```

Vitest: **5 files / 41 tests passed**, including four new static-roadmap tests.
Production build: **3 entries, 28 local references under /oxidj/, script-free teaser**.
The build verifier checks eight statically emitted milestones and no unresolved markers.

`cargo xtask ci`: exit 0:

```text
=> ci summary: layers=ok fmt=ok clippy=ok check=ok  [all pass]
```

## Real-browser verification

Production server: `bun run preview --host 127.0.0.1 --port 4174 --base /oxidj/`.
Puppeteer/Chromium checked `/oxidj/`, `/oxidj/roadmap.html`, `/oxidj/dossier.html`:

- HTTP 200 and no horizontal overflow at 320, 375, 768, 1024, 1280, 1440, 1920px.
- Both redesigned heroes show headline/action at 1280×800.
- Both redesigned routes: Tab reaches the visible outlined skip link; every in-page
  navigation target exists; no page exceptions or console/resource errors.
- JavaScript off: all eight roadmap milestones remain present, with production caveats;
  teaser content remains readable. No reveal hides essential content.
- Axe WCAG 2 A/AA and 2.1 AA: **zero violations**, both redesigned routes at 1440 and 375px.
- Local photograph loads on both routes. Desktop computed animation is `photo-depth`;
  reduced-motion computed animation and transform are both `none`. Mobile is static.
- Dossier retained and checked for route/resource/overflow regressions. Its pre-existing
  default favicon request is excluded; the redesigned routes have an explicit local icon.

Final production captures: `.hallmark/final/{home,roadmap}-{1440,375,1280}.png`,
`{home,roadmap}-fold.png`, and `{home,roadmap}-photo.png`.
Machine results: `.hallmark/final/browser-results.json`. Capture tooling/dependencies stay
outside the site's package manifest. Initial tooling was accidentally traversed by local
format/lint; moved it outside `site/`, restored the untouched dependency manifest/lock,
and reran the real checks. No third-party tooling is committed.

## Hallmark rendered sweep and independent reviews

The installed skill lacks executables. Retrieved upstream Nutlope/hallmark at
`7ec868225c6caa040558c321ee0fefa0e73f83a8` into workspace-local review tooling.
Ran the actual `sloplint.mjs`, not a substitute report, with `HALLMARK_PUPPETEER` pointing
to external browser dependencies and `CHROME_PATH=/usr/sbin/chromium`.

Source/development render sweep, each HTML plus its stylesheet, `--genre atmospheric
--render --url http://127.0.0.1:4173/<route>`: **0 FAIL, 0 open REFLEX, 0 WARN**, exit 0
on both pages. Production output is separately exercised above; its minified CSS strips
the source design stamp, so stamp-based render bookkeeping uses the development route.

Waivers: compact ruled spacing (59), roadmap's single status-summary band (77), and
long-document item headings (78; unused in the last sweep). Existing dense evidence
copy reports as notes under the redesign's content-preservation policy, not as a claim
that the page meets an 80-word marketing budget.

Hero review (fresh subagent): **fix**, three substantive findings, all resolved:
mobile diagram text reflowed, M3 qualification raised, stale acceptance sentence corrected.
Finish/source review (fresh subagent): **ship**, no actionable defects; appropriate to
merge if gates pass. Independently reran 41 tests, typecheck, lint, build and whitespace
check. Old strength noted: CLI proof was previously above the fold; it is still present.

Judged finish: no fake metrics or GUI, no hidden reveal, one button recipe/container,
one photo-scroll pattern, instant focus, tokenized palette/type, section rhythm by rules
and surfaces. Source evidence intentionally stays more detailed than marketing copy.

## Material handoff

No empty slots or missing material block release. The only stock asset is a credited,
local 1280×320 vinyl crop used in shallow strips, subordinate to the library diagrams.
A future owner-approved capture of the real integrated library using synthetic audio
and public-safe paths would add more product evidence; no private library data was used.
