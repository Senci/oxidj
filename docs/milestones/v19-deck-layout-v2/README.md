# v19 — Deck and mixer layout v2 (owner-approved direction)

Status: **design direction approved by the owner (2026-11-10); deck panel v2, mixer v2 and Layout B (default) implemented; arrangements A, C, D still queued.**
Source of truth: [`mockup.html`](mockup.html) (static, synthetic placeholder
content, open it in a browser) and the four renders next to it. This is a
layout study, not the application: nothing here is shipped, and no vendor,
hardware or accessibility qualification is claimed.

Default arrangement is **Layout B**. A, C and D are alternatives the owner may
still switch on later; they are out of scope until Layout B ships.

| File | Arrangement |
|---|---|
| `layout-b.png` | Waveforms and overviews on top; decks with the mixer between them below (default) |
| `layout-a.png` | Decks and centre mixer on top; waveforms and overviews below |
| `layout-c.png` | Waveforms on top; two decks with the mixer docked at the right |
| `layout-d.png` | Mixer beside the waveforms; both decks in a row below |

## Approved decisions

**Waveforms**
- One merged detail waveform: deck A grows up from the centre seam, deck B down,
  one shared needle.
- Two full-track overviews, one per deck, each fitted completely in the pane and
  sharing one needle x position (the tracks' starts and ends may differ).

**Deck panel** (each deck, the same on A and B)
- One content band: the cover's top, the track metadata's top, the BPM's top and
  the pitch fader's top share one edge; the cover's bottom, the hot-cue pads and
  Play button's bottom and the pitch fader's bottom share one edge.
- Square cover on the left, enlarged album art blurred behind the deck.
- Loop in / out / repeat above Cue, Cue above Play. Eight square hot-cue pads
  (4 x 2), MIDI-pad styling, labelled.
- Pitch column: large BPM number (no "BPM" label), SYNC beside key lock, KEY,
  time remaining, tempo-range button; vertical fader spanning the full band.
- Cue = `circle-dot`, Play = `play`; each icon is drawn in its button's colour.
- Deck identity by temperature: A warm (amber), B cool (cyan).
- Mirror option: deck B puts the pitch fader nearest the mixer.

**Mixer**
- Default configuration: EQ knobs outside the faders. Per channel an outer
  column: input selector (optional), A/B label (optional), TRIM (small purple
  square), HIGH, MID, LOW (small gaps above and below this group), FILTER
  (larger), headphone-cue icon. Labels sit above knobs.
- Volume faders and the two meters fill the full height of the channel area and
  share top and bottom edges; the meters sit between the faders.
- Crossfader sits below the faders under a horizontal line, with a CROSSFADER
  label and A / B ends. Optional.
- Standard configuration (knobs above): knobs on one vertical axis, label beneath
  each knob, small gaps around the EQ group.
- Compact and tight: about 11 % of the screen width by default (ultra compact
  about 9.5 %, roomy about 14 %).

**Preferences to persist in Settings** (all in the mockup's sticky bar)
deck theme (Club standard / OxiDJ), mixer density, track metadata, crossfader,
mixer A/B labels, input selectors, knobs outside faders (default on), mirror
deck B.

**Icons**: [Lucide](https://lucide.dev) (ISC licence, Feather-derived parts MIT),
used for lock, play, circle-dot, log-in, log-out, repeat, headphones, timer,
search, settings and panel buttons. No emoji in the interface.

## Not decided / deferred

- Alternative arrangements A, C and D as selectable layouts.
- Real playback-driven overview fitting (shared playback-time scale); the
  mockup's overview alignment is a static illustration.
- Native display, accessibility and performance qualification (NOT DEMONSTRATED).

## Implemented

- Mixer v2: [`app-mixer-v2.png`](app-mixer-v2.png) is a real Bot Screen capture of the application (synthetic scratch catalog, decks empty, detail waveform hidden to show the mixer; compact density at 1440 px wide). Input selectors are a painted placeholder; the master/cue controls sit in a column beside the mixer block.
- Layout B (default arrangement): [`app-layout-b.png`](app-layout-b.png) is a real Bot Screen capture (synthetic scratch catalog, both decks loaded, 1440x791 window, OxiDJ deck theme, mirrored deck B, A/B labels and input selectors on, mixer shown). Detail waveform and the two overviews sit on top (one static needle x across both overviews); deck A, mixer and deck B share one row below. New Settings (Deck view tab, all persisted): deck theme (Club standard / OxiDJ: panel corner radius), track metadata, crossfader, mirror deck B, alongside mixer density, A/B labels, input selectors and knobs outside. The deck area scrolls so the library table keeps at least 120 px. Live playback-driven overview fitting and arrangements A, C, D remain deferred; native display and accessibility qualification NOT DEMONSTRATED.
