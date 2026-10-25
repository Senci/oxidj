# Teaser verification

Local verification on 2026-10-25. This is not a live deployment record.

## Hallmark decisions and reviews

- Dealt Terminal, Ink and Bloom using upstream Hallmark at
  `7ec868225c6caa040558c321ee0fefa0e73f83a8`. Inspected all three theme pictures;
  chose Ink 2/3. Draw `bb241f`: Conversational FAQ, Stage, edge-aligned navigation,
  letter-close footer. No previous project Hallmark build log was present.
- Real material: `idea.md`, `plan/MVP.md`, the dated roadmap snapshot, and actual
  `cargo run -q -p oxidj-cli -- --help` output (exit 0). The CLI excerpt omits arguments
  and says so before the excerpt. No product mock, photograph or private-path GUI capture.
- Hero fresh-context review: **fix**, two findings. Increased CLI text from 14px to 20px;
  moved attribution/argument qualification before the commands. Recaptures inspected at
  desktop and 375px; both resolved.
- Finish fresh-context review: **fix**, two findings. Replaced the middle repeated row
  layout with three unboxed columns, collapsing at mobile; made generic headings specific
  to OxiDJ and catalog acceptance. Final full-page captures inspected; both resolved.
- Theme adaptation: neutrals use a slight cool tint to meet the editorial chroma floor;
  no motion beyond link/control state. Repository links replace an unavailable release
  download. No waivers, missing facts or image slots.

## Executed checks

From `site/`: `bun run typecheck`, `bun run lint`, `bun run test`, `bun run build`,
`bun run format:check`. Exit 0. Vitest: **4 files, 37 tests passed**.
Build emits `dist/index.html`, `dist/roadmap.html`, `dist/dossier.html`.
Build verifier: **3 entries, 25 local references under /oxidj/, script-free teaser**.

From repository root:

```sh
HALLMARK_PUPPETEER="$PWD/.hallmark-tooling" CHROME_PATH=/usr/sbin/chromium \
  node .hallmark-tooling/skills/hallmark/scripts/sloplint.mjs \
  site/index.html site/src/styles/teaser.css --genre editorial --render \
  --url http://127.0.0.1:4174/oxidj/
```

Production preview: `bun run preview --host 127.0.0.1 --port 4174 --base /oxidj/`.
Render result: **0 FAIL, 0 REFLEX, 0 WARN**, exit 0. Four-size hero stage also passed.
The render tier produced desktop folds (1280, 1440, 1920) and desktop/mobile full pages.
Final captures are local ignored files in `site/.hallmark/shots/` and attached to the handoff.

A local Puppeteer smoke against production preview verified:

- `/oxidj/`, `/oxidj/roadmap.html` (eight rendered milestones), `/oxidj/dossier.html`;
- no horizontal overflow at 320, 375, 768, 1024, 1280, 1440 and 1920px;
- Tab focuses the visible skip link with an outline;
- four main teaser sections and acceptance caveat remain with JavaScript disabled;
- no page exceptions or local 4xx/5xx responses (favicon excluded).

`cargo xtask ci`: exit 0,
`=> ci summary: layers=ok fmt=ok clippy=ok check=ok  [all pass]`.
`git diff --check`: exit 0.

## Judged gates and finish review

- 6b/44: left-weighted Stage, capped container; head, lede, action and CLI proof fit at
  1280x800; 1440 and 1920 folds inspected. Mobile naturally stacks.
- 8/9/32: no previous project build to repeat; current capability rows, ownership columns
  on a paper change, acceptance rows, and a letter close vary the rhythm.
- 11/16/23/29/38/47: no card hover theatre, toast, glow, accented heading word, drawn
  window or fake UI. Accent is a focus/underline signal, not a filled surface.
- 46/54/58/63/66/69/70: no invented evidence, eyebrows, decorative dots, glued arrows,
  badges, fake submission or stock signup. Copy stays specific and bounded.
- 77/80/81/82/83/84/85: full-width lists/columns provide mass; actual CLI text replaces
  private-path screenshots. No photo wall, empty slots, counting marks or newsletter close.
- Finish: balanced headings, one primary recipe/container, no skipped heading levels,
  three weights maximum, themed browser surfaces, no justified prose, no hidden mobile
  content. Forms, tooltips, pricing, avatars, video and slots are absent, so their checks
  are not applicable. External fonts have CSS fallback faces.

## Deployment and material handoff

Pages workflow builds and uploads `site/dist` on relevant master pushes or manual dispatch.
The repository needs GitHub Actions selected as its Pages source. Nothing was pushed and
no Actions deployment or public URL readback was performed; deployment is **unverified**.

No replacement slots or missing material block shipping. The most valuable future material
is a clean, owner-approved capture of the integrated library using synthetic files and
public-safe paths, after UI acceptance. It should replace the CLI proof only when real and
legible, never with a drawn interface.
