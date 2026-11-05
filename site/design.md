# OxiDJ public Pages design

Scope: home, roadmap, dossier and styled 404. Preserve the owner-directed dark system.

- Epilogue body/display and an IBM Plex Mono subset (renamed OxiDJ Mono per its OFL
  Reserved Font Name) for code and labels, self-hosted WOFF2 with OFL files.
- Near-black `#101016`, lilac primary actions and cyan links/focus. Palette and font
  families live in root tokens; `night.css` supplies foundations, `public.css` compositions.
- One capped centred container, compact Banner heroes, immediate keyboard focus and
  44px minimum interactive targets. Essential text is available without JavaScript.
- The home hero draws a synthetic 3-band waveform on a beat grid (illustration only),
  then the cropped after-W9 prototype capture tilts flat on scroll. Every screenshot is
  captioned provisional/synthetic and opens in the zoomable lightbox.
- One shared build-time Status callout per public content page, with working local
  features, pre-release maturity and unfinished GUI/accessibility/compatibility/scale work.
- `src/data/roadmap.ts` owns focus and snapshot date. Shared header/footer and metadata
  render through `src/logic/siteMarkup.ts`. The dossier has the same chrome and facts.
- Stable `#m0` through `#m8` anchors. All nine milestones share one timeline with
  their status label shown; the home page repeats them as a compact rail.
  Recent progress is a short change log with exact source files/sections or commits.
- Public prose avoids internal gate terminology and distinguishes Rekordbox XML from
  OneLibrary. Unverified compatibility is never presented as shipped functionality.
- The dossier keeps a deliberately black redaction bar and a local-only interactive joke.
  It is linked only in the footer as Classified file (just for fun).
- No stock photography, invented product screens, metrics, or testimonials. No external
  font calls or runtime dependencies. Motion is transform/opacity only and is removed
  entirely under reduced motion.

## Documented Hallmark exceptions

Compact ruled documentation spacing (59), text-led timeline/CLI sections (77), compact
Banner rather than full-screen fold filling (79), explicitly requested absolute-black
redaction (7b), and the always-available joke button with no disabled state (26b).
The CLI section deliberately keeps runnable commands beside their prerequisites; its
word-density note is recorded, rather than claiming an 80-word marketing-screen budget.
