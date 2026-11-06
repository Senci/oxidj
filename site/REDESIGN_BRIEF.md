# OxiDJ Pages: a library with a little rhythm

Documentation-only design and copy brief, 2026-10-31. Baseline: local master
`e2fd41adf149cdc1899a5948173420fd53358853`, including content refresh `43f4b6d`.
Luna implements this direction in task `t_13e75511`; Sol remains review/docs-only.
This document changes no site behavior and authorizes no push or deployment.

## 1. Direction and material

**Recommendation:** make the site feel like a thoughtfully annotated record sleeve,
not a simulated DJ application. Keep the real prototype in the opening view; add a
small, explorable reference-flow graphic below it. Let visitors uncover how the
library works, rather than watch decorative motion on arrival.

Audience: DJs curious about their local collections and developers considering the
source. Primary job: understand what works, then try the CLI from source. Tone:
plainspoken, curious, lightly mischievous. Keep the dossier's silliness out of the
product's factual promises. The trade-off is deliberate: less spectacle, more trust.

### Pre-flight: what stays and what is new

- **Locked system:** [design.md](design.md), especially its shared Status, compact
  Banner heroes, fonts, palette, stable milestone anchors and prototype caveat.
  Keep Tokyo-night near-black, lilac/purple primary and cyan secondary. Reuse the
  declared tokens in [night.css](src/styles/night.css) and
  [public.css](src/styles/public.css); no palette/theme draw or font replacement.
- **Real material:** four current route templates, a synthetic-library prototype
  image, source-backed milestones and three dated updates. No real music collection,
  metrics, quotes, testimonials, waveform, or performance-product screen is supplied.
- **Current imagery:** [image provenance](public/images/README.md) records the
  800×600 prototype and 1200×630 hero-derived social image as project MIT material.
  Preserve their local paths and notices, plus self-hosted font/OFL notices. The
  screenshot shows one synthetic `fixture.flac`, search/sort and selected-track facts;
  it is not an interactive product demo or a finished desktop interface.
- **Historical record, not current markup:** accepted redesign task `t_a64c36bb`
  (PR [#21](https://github.com/Senci/oxidj/pull/21)) introduced a reference-flow
  diagram and local Pixabay photo/parallax. Today's [home](index.html) no longer
  contains that diagram or photo; old flow/parallax rules remain in `night.css`.
  The image README explicitly records the photo's removal. Do not restore a retired
  photo, claim its credit is still present, or call unused CSS a visible animation.
  Preserve any extant diagrams/assets and their provenance; the flow below is a
  proposed new explanation using the accepted diagram's reference-only meaning.
- **New work proposed:** route-specific composition, the copy substitutions below,
  one native detail-reveal interaction and short, explicitly triggered vector accents.
  No new runtime dependencies, remote assets, trackers or network services.

**Hallmark picks:** locked Tokyo night; atmospheric material with documentation
clarity; shared index navigation/project colophon; compact Banner opening;
signature: original audio stays outside the catalog while references connect it
to the viewer. Home explains, roadmap records, dossier plays, 404 helps.
Keep existing documented compact-spacing/text-density exceptions; do not manufacture
six marketing sections or pad a short route to a full-screen stage.

## 2. Facts that the redesign must not soften

Keep [roadmap.ts](src/data/roadmap.ts) as the one source for snapshot date, focus,
working/unfinished Status text, milestone states and updates. The date describes a
content snapshot, not a release. Do not insert a second handwritten status into HTML.

| Public fact / boundary | Canonical source and implementation consequence |
| --- | --- |
| Bounded local M1–M3 work is complete; the viewer remains provisional. | [Local closeout](../plan/MVP.md#bounded-local-m1m3-acceptance-closeout-2026-10-26), [status](../plan/STATUS.md), [viewer report](../prototypes/oxidj-library-ui/REPORT.md). Keep “Local scope complete” and “Prototype tested,” not “Desktop released.” |
| The local, single-process catalog is capped at 10,000 tracks. | [Local contract](../plan/MVP.md#revised-local-m1m3-acceptance-contract-v1-2026-10-23). This is a scope limit, not a measured capacity or speed claim. |
| Import stores references, filenames and technical facts; supported WAV/FLAC/MP3 encodings have limits. Originals remain unchanged. | Same local contract and [current CLI guide](index.html#try-it). Never imply embedded DJ-tag/artwork ingestion, editing or playback in this local slice. |
| M4 remains in progress: synthetic Rekordbox XML tests, preview, supported-subset export to a new file, edits checking expected old values. Unsupported content can prevent export. | [Current M4 text](src/data/roadmap.ts), [adapter/tests](../crates/oxidj-rekordbox), [status](../plan/STATUS.md). Do not imply every XML file can be exported or that a website interaction edits a library. |
| XML is not OneLibrary. Real Rekordbox/OneLibrary compatibility has not been demonstrated. | [Format research](../docs/research/interoperability.md#findings-and-confidence). No hardware-success animation, vendor logo endorsement, “lossless,” “sync,” or “ready for your USB.” |
| Production desktop, accessibility/platform testing and 500,000-track/60fps performance testing remain pending. | [Local contract](../plan/MVP.md#revised-local-m1m3-acceptance-contract-v1-2026-10-23), [current unfinished text](src/data/roadmap.ts). Website accessibility checks do not prove application accessibility. |
| Linux is the exercised development platform; macOS/Windows are unverified. Instructions are CLI-from-source, not a packaged release. | [Home prerequisites](index.html#try-it). Keep prerequisites beside the commands; do not add a Download or Launch app CTA. |
| Decks and wider performance features are later ambitions. | [Vision and exclusions](../idea.md#mvp). No fake transport, audio meters, beat matching or playback demonstration. |

Keep the accepted unfinished paragraph **verbatim** in the shared Status:

> Compatibility with real Rekordbox or OneLibrary libraries has not been demonstrated. Production desktop, accessibility and platform testing is still pending, as is performance testing at 500,000 tracks and 60 frames per second.

Keep “Status: pre-release,” the current-focus link and the exact snapshot date. Keep
this Status visible on home, roadmap and dossier, outside disclosures and animations.
Keep all M4 limits in its milestone even though the shared Status also exists. Public
copy should explain limits in ordinary language, not bring internal test/gate names
onto the site. The facts table above is implementation guidance, not page copy.

## 3. Route-by-route copy and hierarchy

Only the substitutions explicitly listed below are recommended; retain the remaining
facts, commands, prerequisites, evidence links, dates, jokes and notices. Sentence
case, untracked labels, body face for navigation/captions. No eyebrows, motivational
filler, decorative status dots, arrow-suffixed links or invented proof.

### Home: `./` (`index.html`)

**Order:** compact heading/CTA plus real prototype; shared Status; why this library;
reference-flow explainer within that section; XML/OneLibrary distinction; CLI guide;
shared footer. Keep the prototype as a real image, not a card containing a drawn OS
window. Do not add a competing second hero or a sticky marketing CTA.

| Current text | Recommended text / disposition |
| --- | --- |
| “A library before the decks.” | Keep. It is short, memorable and honest about scope. |
| “A Rust-first DJ library, starting with your local files.” | “Your local DJ library, built in Rust.” |
| “Try the catalog” | “Try the CLI” with the same `#try-it` destination. Clarifies what the click actually does. |
| “See the roadmap” | Keep its `./roadmap.html` destination. |
| “Prototype” image caption | “Prototype viewer with synthetic audio.” Preserve the descriptive alt text and provisional meaning. |
| “Your collection, beyond one ecosystem.” | “Your files stay yours.” |
| Paragraph beginning “Moving between DJ tools…” | “Playlists, cues and beat grids can get stuck in one tool. OxiDJ aims to make moving them safer, without quietly losing the work behind a set.” |
| Paragraph beginning “For DJs with large local collections…” | “We’re starting small: audio stays where it is, while references and technical facts live in a local catalog. No cloud account needed. Dependable exchange comes before decks.” |
| “XML first. Hardware exchange later.” | Keep, together with its full format distinction and research link. |
| “Try it from source.” | “Try the CLI from source.” |

Keep the M1/M2/M3 preview links and their descriptions; they are useful stepping
stones, not three new product promises. Leave runnable command blocks unchanged.
Do not collapse platform warnings, the 10,000-track limit, encoding/exclusion notes
or expected empty output. These are part of trying the software safely, not footnotes.

**New explainer text:** heading “How the local library fits together.” Caption:
“Audio stays in place. The catalog keeps references and technical facts.” Use three
stages: **Audio files**, **Local catalog**, **Prototype viewer**. Not an upload flow.
Its detail text is specified in section 4. Source: the local contract and viewer report.

### Roadmap: `./roadmap.html`

**Order:** short opening and current-focus note; shared Status; M0–M4 ruled timeline;
M5–M7 Later; recent dated progress; shared footer. Let M4 lead through its existing
focus link and cyan rule, not a completion percentage, countdown or estimated date.

| Current text | Recommended text / disposition |
| --- | --- |
| “The road to a trustworthy library.” | “What works. What comes next.” |
| “Local files first. Careful exchange next. Decks come later.” | “Local files first. Careful exchange next. Decks later.” |
| Summary beginning “M0 to M3 cover…” | “M0 to M3 cover the local-library foundation. M4 is the exchange work now in progress. These are development stages, not release dates.” |
| “Milestones,” “Later,” “Recent progress” | Keep: useful, plain labels. |
| “Added an exact-value guarded Track Kind edit.” | “Track Kind edits check the old value first.” Keep its date, synthetic-only detail and exact source link. |

Keep the other two recent updates and all eight milestone facts/status labels.
Retain the full accepted `currentSummary` and M4 detail; do not shorten away the
synthetic-test scope, new-file destination, old-value checks or export limitations.
Render technical detail as readable text next to sources, not hover-only tooltips.
The timeline is an ordered development record, not a visual measurement of progress.

### Dossier: `./dossier.html`

**Order:** compact classified-file opening/redaction; real shared Status; dossier
fields; evidence log; local joke response/button; shared footer. Keep this route
footer-only as “Classified file (just for fun).” No navigation promotion or real
surveillance/security claim. The black redaction remains intentional and unchanged.

| Current text | Recommended text / disposition |
| --- | --- |
| “Classified file 0042.” | Keep. |
| “Subject: your library. A completely unserious investigation.” | Keep: the joke announces itself. |
| “Live analysis” | “Observations, entirely unserious.” |
| “Observation pending. The archive listens.” | “Observation pending. This joke stays in your browser.” |
| “Request asylum” and existing rotating denials | Keep the label, response cycle and local behavior. Do not reinterpret these as real submissions. |

Add one plain helper sentence beside the joke: “Nothing is sent or saved; this only
changes the text on this page.” This describes the current
[local handlers](src/ui/app.ts), not an analytics promise about an external service.
Retain the no-JavaScript explanation, `aria-live="polite"` response and all joke DOM
hooks. Do not put pointer/keystroke observation updates into a live region or animate
every mouse movement. The humor comes from the writing, not flashing redactions.

### Not found: `404.html`

**Order:** shared navigation; clear missing-page heading, one helpful sentence, home
and roadmap recovery links; shared footer. No progress/status panel is needed here.

| Current text | Recommended text / disposition |
| --- | --- |
| “File missing.” | “This page isn’t in the collection.” |
| “This page has left the collection. Let's find your way back.” | “No page here. Your next stop can be home or the roadmap.” |
| “Return home” / “Read the roadmap” | Keep both destinations and button/link hierarchy. |

A static broken-reference line can echo the home diagram if space permits. No
animated failure, fake retry, automatic redirect, search box without a backend or
“your files are missing” message. The error concerns a web page, not the DJ catalog.
Keep the `/oxidj/` base, noindex and nested-missing-path resource/navigation behavior.

## 4. Motion and interaction: follow the reference

One coherent motif: a short cyan reference segment moves between fixed lilac/neutral
stages. This is an explanatory vector graphic, never an audio waveform, transfer
progress meter or live activity indicator. **Nothing autoplays**, including on load,
scroll, intersection, hover or focus. No sound, video, loops, cursor followers,
confetti, page transitions or parallax. Do not activate the unused photo rules.

### Home: an explorable reference flow

- Keep the prototype in the hero; place the compact graphic inside the following
  explanation section. Desktop: three stages across; mobile: three stages stacked.
  Original files remain visibly outside the catalog boundary. Do not animate files
  entering the database; only the reference segment travels.
- Use native `details`/`summary` entries beside/under the static graphic, with these
  summaries and panel text:
  - **Audio files:** “The importer reads supported audio encodings. Originals stay
    in place and unchanged.”
  - **Local catalog:** “The catalog stores file references, filenames and technical
    facts. This local scope is single-process and capped at 10,000 tracks.”
  - **Prototype viewer:** “The provisional viewer reopens the saved catalog for
    search, sort, selection and track details.”
- These disclosures are the meaningful interaction: selecting a stage reveals its
  role, using only already-supported facts. All three may remain open; no custom
  tabs/arrow-key widget, mandatory sequence or hidden source links. Use ordinary
  text in HTML rather than shrinking labels inside SVG. Mark redundant SVG art
  decorative, so it does not duplicate the accessible explanation.
- Progressive enhancement may add a quiet **“Trace the references”** button after
  its handler initializes. One explicit activation plays a 600–800ms finite
  sequence; the graphic then rests in the same complete state. No moving controls
  or text. Re-activation restarts that single sequence, not overlapping effects.
  A click does not import, write, play or request any file. Include “Illustration
  only” beside the control, not a success notification. With scripts off the static
  diagram and native disclosures still work; the dead motion button is absent.

### Roadmap: a fixed timeline with a deliberate trace

Keep the existing semantic list and all anchors. Add a narrow vector rail alongside
M0–M4; labels and source links remain outside the rail. A progressively enhanced
**“Trace the milestones”** button may run one finite 600–800ms accent along that
rail. Stop at M4's existing in-progress boundary, never fill M4 or light up Later as
complete. This does not calculate percent done. Static labels carry all meaning;
no automatic scroll, animated counters, timeline scrubbing or hidden milestones.
Prefer this small shared motif over a second animation system.

### Dossier and 404: resist the extra show

On an explicit asylum-button activation only, the response may receive one 120–180ms
opacity/2px settle. Update the text immediately, keep focus on the button and leave
room for multiline responses; no typewriter delay or removal of earlier page facts.
The redaction never reveals personal content. Keep all current observation logic
local, with no new listeners for motion or telemetry. The 404 stays still: successful
recovery navigation is its interaction.

### Complete motion fallback and interaction contract

| Situation | Required behavior |
| --- | --- |
| Default/load/scroll/hover/focus | Complete static art and visible content. No automatic animation. Focus is immediate and never starts a trace. |
| Explicit trace activation | One finite transform/opacity sequence on decorative segments only. No layout animation, flashing, or recurring timer. |
| Reduced motion at load | No animation, transitions, smooth scrolling, parallax, translation or response settle. Native disclosures and joke text update instantly. Hide optional trace-only buttons; no information is removed. |
| Reduced motion enabled during a sequence | Cancel the running sequence, restore its complete static state, remove any motion-only controls without stranding focus (return it to a stable nearby heading if necessary). No replay when the preference later changes back. |
| JavaScript unavailable or initialization fails | Static diagram/timeline and every factual/source link remain visible. Native disclosures work. No inert trace controls. Dossier retains its existing script-free explanation. |
| Animation API/CSS enhancement unavailable | Static final state; optional controls must not promise an effect that cannot run. No polyfill/dependency needed. |

Use existing CSS/SVG/TypeScript only. Prefer transform and opacity on pre-drawn
segments, not animated widths/heights/positions/stroke drawing. Declare motion values
with the theme; never `transition: all`. A button's default, hover, held press and
focus-visible appearances must be distinguishable, and text must remain legible.
Native disclosure state needs a visible open/closed cue besides color. Avoid disabled
states unless a real state requires one; do not introduce artificial loading.

## 5. Mobile, keyboard and preservation rules

- Keep the capped centered container and growing desktop gutters. At 390px use one
  column, readable body text (at least 16px), no pinned panels or sideways timeline.
  Use `minmax(0, 1fr)` for image tracks and min-width/word wrapping for headings.
  Keep `overflow-x: clip` on both document/root body; do not hide actual overflow.
- Preserve the prototype's 4:3 ratio, full image and caption. Do not crop away the
  filename/details to pretend it is a polished dashboard. The diagram must not push
  the headline, sentence and primary action below the 1280×800 opening view.
- All actions/disclosure summaries have at least 44×44px targets. Use native buttons
  for actions, links for destinations, and summary for disclosure. Logical Tab order;
  Enter/Space opens disclosures and activates buttons. No drag, hover or mouse-only
  information; no focus trap, automatic focus on reveal or animations on focus.
- Preserve the immediate cyan focus-visible ring, skip-to-content link and heading
  structure. Announce only the dossier response through its existing polite region;
  trace animation has no live announcements. Contrast must meet WCAG AA (4.5:1 normal
  text; 3:1 large text and meaningful UI boundaries). Do not rely on purple/cyan alone.
- Keep all four paths, existing link destinations, source links and all IDs. This
  includes `#main`, `#title` where present, home `#why`, `#exchange`, `#try-it`,
  `#try-title`; roadmap `#roadmap`, `#milestones-title`, `#m0`–`#m7`, `#later-title`,
  `#updates`, `#updates-title`; dossier section IDs and `#analysis-output`,
  `#asylum-button`, `#asylum-output`; shared `#status-title`. New IDs must be unique.
  Preserve build-time fragment hooks, metadata/canonical/social wiring and 404 base.
- No remote fonts, new runtime packages, external scripts, hidden requests, storage,
  cookies or private media. Use escaped build-time text/current safe DOM updates;
  never introduce arbitrary HTML insertion for diagram labels or joke responses.

## 6. Implementation handoff and acceptance

This brief is the only output of task `t_f035ffd9`. No code/assets/design-system file
is changed here. Do not run a theme scaffold or generate auxiliary Hallmark files for
this documentation task. No rendering or animation pass is claimed by this document.

For Luna's implementation, preserve component ownership: route markup owns its
composition; shared Status/nav stays in [siteMarkup.ts](src/logic/siteMarkup.ts);
facts stay in `roadmap.ts`; the existing dossier logic keeps its local behavior.
Extend the existing styles/handlers only as needed; no wholesale component replacement
or asset deletion is proposed. Update implementation documentation only to describe
what actually ships, not to make this proposal a completed feature.

Minimum independent review for the code follow-on:

1. Compare route-by-route copy, all existing anchors/links, notices and maturity
   caveats against the baseline; verify M4 cannot read as a completed integration.
2. Exercise disclosures with Tab/Enter/Space, trace controls by keyboard and pointer,
   repeated activation and focus visibility. Verify the joke still cycles local text.
3. Inspect actual captures on all four routes at 390px and 1440px. Check the opening
   fold, static diagram semantics, image readability, resources, contrast, overflow,
   console, skip links, nested missing-path 404 and source anchors.
4. Check no autoplay on load/scroll/focus; test reduced motion before load and while a
   trace runs, script-free content/disclosures and static enhancement-failure behavior.
5. Run the existing site typecheck, lint, tests, format check and build plus
   `cargo xtask ci`, as required by the implementation card. Extend focused behavior
   tests for the new interaction, not snapshots that merely approve changed wording.
6. Review only changed site code for unsafe HTML/data handling and unexpected network
   activity. Preserve unrelated working-tree changes. Commit only the scoped work;
   no deployment is authorized by this brief.

No new material is required to build this direction. The existing prototype slot is
800×600 (4:3), captioned as provisional; the social slot is 1200×630 derived artwork.
A future owner-approved, genuinely exercised viewer capture would improve that slot
most. Until then retain the actual supplied image, rather than inventing an interface.
