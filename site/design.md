# OxiDJ public Pages design

Scope: home, roadmap, dossier and styled 404. Preserve the owner-directed dark system.

- Epilogue body/display and IBM Plex Mono for real CLI code, self-hosted with OFL files.
- Near-black `#101016`, lilac primary actions and cyan links/focus. Palette and font
  families live in root tokens; `night.css` supplies foundations, `public.css` compositions.
- One capped centred container, compact Banner heroes, immediate keyboard focus and
  44px minimum interactive targets. Essential text is available without JavaScript.
- The home hero uses the existing synthetic-library screenshot, captioned Prototype.
  It is a provisional viewer, not a promise of the production interface.
- One shared build-time Status callout per public content page, with working local
  features, pre-release maturity and unfinished GUI/accessibility/compatibility/scale work.
- `src/data/roadmap.ts` owns focus and snapshot date. Shared header/footer and metadata
  render through `src/logic/siteMarkup.ts`. The dossier has the same chrome and facts.
- Stable `#m0` through `#m7` anchors. M0-M4 are compact timeline rows; M5-M7 share Later.
  Recent progress is a short change log with exact source files/sections or commits.
- Public prose avoids internal gate terminology and distinguishes Rekordbox XML from
  OneLibrary. Unverified compatibility is never presented as shipped functionality.
- The dossier keeps a deliberately black redaction bar and a local-only interactive joke.
  It is linked only in the footer as Classified file (just for fun).
- No stock photography, invented product screens, metrics, or testimonials. No external
  font calls or new runtime dependencies. Motion stays optional with reduced-motion support.

## Documented Hallmark exceptions

Compact ruled documentation spacing (59), text-led timeline/CLI sections (77), compact
Banner rather than full-screen fold filling (79), explicitly requested absolute-black
redaction (7b), and the always-available joke button with no disabled state (26b).
The CLI section deliberately keeps runnable commands beside their prerequisites; its
word-density note is recorded, rather than claiming an 80-word marketing-screen budget.
