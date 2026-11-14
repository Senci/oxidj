export interface SourceLink {
  label: string;
  href: string;
}
export interface Milestone {
  id: string;
  title: string;
  status: 'complete' | 'in-progress' | 'planned';
  statusLabel: string;
  detail: string;
  source: SourceLink;
}
export interface Screenshot {
  /** File stem under public/images (native-resolution lossless WebP). */
  id: string;
  width: number;
  height: number;
  /** Half-width lossy WebP for 1x screens, when one exists. */
  halfWidth?: number;
  alt: string;
  caption: string;
}
export interface ProjectUpdate {
  date: string;
  title: string;
  detail: string;
  source: SourceLink;
}
const repo = 'https://github.com/Senci/oxidj';
const closeout = `${repo}/blob/master/plan/MVP.md#bounded-local-m1m3-acceptance-closeout-2026-10-26`;
export const roadmapSnapshot = {
  verifiedAt: '2026-11-12',
  currentFocus:
    'Audio output improvements and safe imported-playlist editing; exchange remains in progress',
  currentSummary:
    'The library window now browses imported nested playlists, shows their stored-order members and loads a selected member explicitly onto deck A or B from its exact imported location. Selection does not load, loading does not autoplay, missing locations are refused, and loading does not edit imported data. The screen capture is synthetic-only. The window also has two playable decks: one merged 3-band waveform with deck A above and deck B below, an audio engine, hot cues and loops, tempo with key lock, SYNC and an optional mixer. The owner listened to the installed build and reports that it works (no measurements); DJ hardware checks are still open. On the exchange side, synthetic Rekordbox XML fixtures support read-only preview, bounded export of a supported subset to a new file, and exact-value guarded edits. On one real library, playlists, track details, beat grids and cues went into a catalog and a new XML file, and the owner confirmed in Rekordbox on Windows that the export and an edited re-export load with tracks, BPM, key, cues, beat grids, playlists and the edits intact.',
  working:
    'A persistent local catalog, reference-only WAV/FLAC/MP3/AIFF import and a provisional viewer with search, sort and track details work today. The library window browses imported nested playlists and their stored-order members; an explicit Load A or Load B action loads the selected member only from its exact imported location. Selection does not load or autoplay; a missing location is refused without changing the deck, and the imported snapshot stays unchanged. This UI is provisionally exercised with synthetic data only. The window has a full-width track list with side panels you can show or hide, and draws a smooth vector waveform (mono, 3-band or RGB) for the selected track with bar and beat lines from an imported beat grid, or from its own computed BPM and beat grid when none was imported. Tracks load onto two decks shown in one merged waveform (deck A warm, above; deck B cool, below) with a separate overview per deck; both draw low, mid and high bands in the deck colours, and the overviews scale quiet tracks up and mark cues, loops and every 16 bars. Every deck section can be collapsed, and Settings (Ctrl+,) keeps the choices. A CPAL audio engine plays both decks with cue, play, eight hot cues and loops; the tempo fader has key lock, SYNC matches tempo and beat phase, and an optional mixer adds trim, 3-band EQ, filter, faders, a crossfader and meters. scripts/install-local.sh builds release binaries and adds a launcher, desktop menu entry and icon for the current user on Linux. From the command line, a Rekordbox USB export can be read into a new catalog and written back out as a checked XML file, with Windows drive letters via --drive and, opt-in, computed beat grids for tracks that have no Rekordbox grid.',
  unfinished:
    'Beyond one owner-run round trip through Rekordbox on Windows, compatibility with real Rekordbox or OneLibrary libraries and DJ hardware (CDJ/XDJ) has not been demonstrated. Computed BPM is a suggestion, not Rekordbox parity, and a computed grid has one constant tempo (no tempo-change maps); Rekordbox has not yet been checked with the exported computed grids. The decks have only had an informal owner listening check, have not been used with club gear, controllers or CDJ/XDJ players, and make no latency guarantee: one 600 s run through PipeWire had 1 audio dropout (xrun), and moving the tempo fader with key lock on can shift the next beat by up to 5 ms. Physical USB reconnect and four-channel cue, Windows ASIO and macOS audio remain untested. Production desktop, accessibility and platform testing is still pending, as is performance testing at 500,000 tracks and 60 frames per second.',
  repo,
} as const;
export const milestones: readonly Milestone[] = [
  {
    id: 'm0',
    title: 'Foundation',
    status: 'complete',
    statusLabel: 'Complete',
    detail: 'Rust workspace, architecture and development checks established.',
    source: {
      label: 'Foundation record',
      href: `${repo}/blob/master/docs/history/status-2026-11-02.md#m0-09-verification-summary-independent-lead-run`,
    },
  },
  {
    id: 'm1',
    title: 'Local catalog',
    status: 'complete',
    statusLabel: 'Local scope complete',
    detail:
      'Stable track IDs, technical facts, persistence, recovery and backup. The current local scope is capped at 10,000 tracks.',
    source: { label: 'Catalog acceptance', href: closeout },
  },
  {
    id: 'm2',
    title: 'Reference-only import',
    status: 'complete',
    statusLabel: 'Local scope complete',
    detail:
      'Recursively import supported WAV, FLAC, MP3 and AIFF/AIFF-C encodings by reference. Originals stay in place and unchanged; filenames and technical facts enter the catalog.',
    source: { label: 'Import acceptance', href: closeout },
  },
  {
    id: 'm3',
    title: 'Library viewer',
    status: 'complete',
    statusLabel: 'Prototype tested',
    detail:
      'Reopen the saved catalog, search, sort, select and inspect tracks. Empty, missing-file and error states have been exercised.',
    source: {
      label: 'Viewer test report',
      href: `${repo}/blob/master/prototypes/oxidj-library-ui/REPORT.md`,
    },
  },
  {
    id: 'm4',
    title: 'Rekordbox / OneLibrary exchange',
    status: 'in-progress',
    statusLabel: 'In progress',
    detail:
      'Synthetic Rekordbox XML fixtures support read-only preview, bounded export of a supported subset to a new file, and guarded edits that check the expected old value. Unsupported content can prevent export. A first import of a real library export, with playlists, track details, beat grids and cues, has been checked against an independent decoder, and Rekordbox on Windows loaded the export in an owner-run check. Broader compatibility with real Rekordbox or OneLibrary libraries has not been demonstrated.',
    source: { label: 'XML adapter and tests', href: `${repo}/tree/master/crates/oxidj-rekordbox` },
  },
  {
    id: 'm5',
    title: 'DJ metadata',
    status: 'complete',
    statusLabel: 'Local UI scope complete',
    detail:
      'Read-only imported cues, loops and key in Details, with imported beat grids displayed in the waveform. Vendor and hardware checks remain open.',
    source: { label: 'Priorities', href: `${repo}/blob/master/idea.md#priorities` },
  },
  {
    id: 'm6',
    title: 'Analysis',
    status: 'in-progress',
    statusLabel: 'First slice',
    detail:
      'Musical and technical analysis workflows. The first piece is a smooth vector waveform view with mono, 3-band and RGB styles, live per-band EQ gain, and bar and beat lines when the imported library has a beat grid. OxiDJ can compute BPM, a beat grid and a key suggestion; imported grid data takes precedence. Across one whole real library (1,317 tracks) its mean tempo was within 2% of Rekordbox on 97% of tracks, with a median beat-tracking score (F70) of 0.84 on the agreeing tracks. That compares one mean tempo per track; it is a suggestion, not Rekordbox parity. The normal computed grid uses one constant tempo; a separate variable-tempo inspector is not used for export or SYNC.',
    source: { label: 'Analysis status', href: `${repo}/blob/master/plan/MVP.md#milestones` },
  },
  {
    id: 'm7',
    title: 'Interoperability proof',
    status: 'in-progress',
    statusLabel: 'First slices',
    detail:
      'Import, manage and export a real library with supported metadata intact. The first import and checked XML export has run on one private library. The owner then loaded the export in Rekordbox on Windows (tracks, BPM, key, cues and beat grids), reimported an export with two OxiDJ edits, imported a playlist and played a track. DJ hardware (CDJ/XDJ) has not been tested.',
    source: { label: 'MVP objective', href: `${repo}/blob/master/idea.md#mvp` },
  },
  {
    id: 'm8',
    title: 'Decks and playback',
    status: 'in-progress',
    statusLabel: 'First version',
    detail:
      'The two decks use one merged waveform (deck A warm, above the centre line; deck B cool, below) with a separate overview per deck, collapsible sections and Settings (Ctrl+,). A CPAL audio engine plays both decks with cue, play, eight hot cues and loops. Tempo has key lock (Signalsmith Stretch), SYNC matches tempo and beat phase and refuses visibly when a deck has no beat grid, and an optional mixer adds trim, 3-band EQ, filter, faders, a crossfader and meters; when hidden it passes audio through unchanged. Detail and overviews draw low, mid and high bands in each deck colour. The owner listened to the installed build: "it works", with no measurements. Not yet demonstrated: club gear, controllers and CDJ/XDJ playback, or any latency guarantee. One 600 s PipeWire run had 1 xrun; a key-locked tempo move can shift the next beat by up to 5 ms. Imported nested playlists are browsable in stored order. Selecting a member only selects it; explicit Load A or Load B loads it from its exact imported location, never by title or filename fallback. Missing locations are refused without replacing the deck, loading does not autoplay, and imported metadata stays unchanged. This UI capture is synthetic-only. Vendor import compatibility, production GUI/accessibility/platform checks, and 500,000-track/60fps performance remain NOT DEMONSTRATED.',
    source: { label: 'Approved imported-playlist UI', href: `${repo}/commit/8068744` },
  },
];
export const projectUpdates: readonly ProjectUpdate[] = [
  {
    date: '2026-11-12',
    title: 'Audio output and safer playlist editing.',
    detail:
      'Output device, buffer and limiter settings, measured output status, JACK support, recovery and smoother key-locked loop transitions are integrated. A muted synthetic JACK run at 48 kHz and 256 frames lasted 600.1 seconds with zero reported xruns or errors; it is not physical hardware validation. Imported root playlist members can be moved safely, preserving duplicates.',
    source: { label: 'Audio status', href: `${repo}/commit/30cd026` },
  },
  {
    date: '2026-11-10',
    title: 'Deck and mixer layout v2.',
    detail:
      'Four selectable deck arrangements, a compact mixer, aligned deck controls and shared playback-time overviews. The reviewed build is installed; native display, accessibility and performance checks remain open.',
    source: { label: 'Layout status', href: `${repo}/commit/5bffdad` },
  },
  {
    date: '2026-11-07',
    title: 'Browse imported playlists and load a member to either deck.',
    detail:
      'The provisional library window shows nested playlists and ordered members. Selection is separate from loading; Load A and Load B use the exact imported location, do not autoplay, and refuse unavailable locations rather than falling back to a title or filename. Synthetic-only UI evidence; vendor compatibility and hardware checks remain NOT DEMONSTRATED.',
    source: { label: 'Imported-playlist UI', href: `${repo}/commit/8068744` },
  },
  {
    date: '2026-11-05',
    title: 'Deck waveforms in three bands.',
    detail:
      'Detail and overviews draw low, mid and high bands in each deck colour. Overviews scale quiet tracks up, dim the played part and mark cues, loops and every 16 bars.',
    source: { label: '3-band deck waveforms', href: `${repo}/commit/2108b79` },
  },
  {
    date: '2026-11-05',
    title: 'Owner listening check: it works.',
    detail:
      'The owner played both decks on the installed build. An informal check with no measurements; club gear and CDJ/XDJ are untested.',
    source: { label: 'Listening check', href: `${repo}/commit/33ba474` },
  },
  {
    date: '2026-11-05',
    title: 'Computed BPM on a whole real library.',
    detail:
      'Mean tempo within 2% of Rekordbox on 97% of 1,317 tracks; median F70 0.84 where they agree. A whole-library comparison, not Rekordbox parity.',
    source: { label: 'Real-library status', href: `${repo}/commit/414d111` },
  },
  {
    date: '2026-11-05',
    title: 'Optional mixer.',
    detail:
      'Trim, 3-band isolator EQ, filter, channel and master faders, a crossfader with three curves and peak meters. Hidden by default; hidden means audio passes through unchanged.',
    source: { label: 'Optional mixer', href: `${repo}/commit/8b6087b` },
  },
  {
    date: '2026-11-05',
    title: 'Key lock and SYNC.',
    detail:
      'The tempo fader keeps pitch with key lock (Signalsmith Stretch). SYNC matches tempo and beat phase, and says why when it cannot, for example a deck without a beat grid.',
    source: { label: 'Key lock and SYNC', href: `${repo}/commit/bdeecf5` },
  },
  {
    date: '2026-11-05',
    title: 'Decks play audio.',
    detail:
      'A CPAL audio engine plays both decks with cue, play, seek, hot cues and loops. One 600 s PipeWire run had 1 xrun; listening is still an owner check.',
    source: { label: 'Audio engine', href: `${repo}/commit/0906ddb` },
  },
  {
    date: '2026-11-05',
    title: 'Two decks in one merged waveform.',
    detail:
      'Deck A warm above the centre line, deck B cool below, one playhead and a separate overview per deck. Every section collapses; Settings opens with Ctrl+,.',
    source: { label: 'Merged decks', href: `${repo}/commit/9b9acbd` },
  },
  {
    date: '2026-11-05',
    title: 'Computed grids can go into the XML export.',
    detail:
      'An opt-in flag writes a computed beat grid for tracks that have none from Rekordbox. Rekordbox loading these is not yet owner-checked.',
    source: { label: 'Computed grids in export', href: `${repo}/commit/d295fc4` },
  },
  {
    date: '2026-11-04',
    title: 'Install script for Linux.',
    detail:
      'scripts/install-local.sh builds release binaries and adds a launcher, desktop entry and icon under ~/.local, without touching library data.',
    source: { label: 'Install script', href: `${repo}/commit/21d652c` },
  },
  {
    date: '2026-11-04',
    title: 'Computed BPM and beat grid when none was imported.',
    detail:
      'The details panel draws bar and beat lines from a computed grid and lists BPM (computed). It is a suggestion: within 2% of Rekordbox on 91% of 90 real tracks.',
    source: { label: 'Computed beat grid', href: `${repo}/commit/9a9b96c` },
  },
  {
    date: '2026-11-04',
    title: 'AIFF and AIFF-C files import and get waveforms.',
    detail:
      'A strict streaming decoder reads big- and little-endian PCM and float AIFF-C. M4A/AAC, ALAC and Ogg/Opus are still unsupported.',
    source: { label: 'AIFF decoding', href: `${repo}/commit/ec1c422` },
  },
  {
    date: '2026-11-04',
    title: 'Denser, calmer library window.',
    detail:
      'Segmented panel toggles, search with a clear button, status dots, right-aligned numbers and aligned track details.',
    source: { label: 'Shell polish', href: `${repo}/commit/339372b` },
  },
  {
    date: '2026-11-04',
    title: 'Edited export reimported by Rekordbox.',
    detail:
      'Owner check on Windows: two OxiDJ edits appear in Rekordbox; playlist import and playback work with a correct beat grid. Hardware is not tested.',
    source: { label: 'Round-trip record', href: `${repo}/commit/c091a13` },
  },
  {
    date: '2026-11-04',
    title: 'Exports load in Rekordbox on Windows.',
    detail:
      'export-library-xml --drive X writes Windows drive-letter locations. The owner saw tracks, BPM, key, cues and beat grids load.',
    source: { label: 'Windows drive letters', href: `${repo}/commit/16f398d` },
  },
  {
    date: '2026-11-03',
    title: 'Tidier waveform panel and steadier deck controls.',
    detail:
      'Gain sliders line up with their values, captions wrap inside the panel and the detail strip uses spare height. Play and Cue no longer move when a deck starts.',
    source: { label: 'Waveform panel polish', href: `${repo}/commit/89cd073` },
  },
  {
    date: '2026-11-03',
    title: 'Compact library window with panels you can hide.',
    detail:
      'The track list spans the window; navigation, details and decks toggle from the toolbar or with Ctrl+B, Ctrl+I and Ctrl+D. The waveform shows bar and beat lines from an imported beat grid.',
    source: { label: 'Compact library window', href: `${repo}/commit/9b2a424` },
  },
  {
    date: '2026-11-03',
    title: 'Waveform view with mono, 3-band and RGB styles.',
    detail:
      'Peaks are computed while streaming audio and stored in the catalog. EQ gains scale the drawing live without re-analysis.',
    source: { label: 'Waveform view', href: `${repo}/commit/8713512` },
  },
  {
    date: '2026-11-03',
    title: 'Beat grids and cues now flow through import and export.',
    detail: 'A read-only analysis-file decoder feeds tempo and cue marks into the exported XML.',
    source: { label: 'Beat grids and cues', href: `${repo}/commit/eac045c` },
  },
  {
    date: '2026-11-03',
    title: 'Genre, label, key, year and more carry through import.',
    detail: 'Values the source cannot supply are counted as missing, never invented.',
    source: { label: 'Track metadata', href: `${repo}/commit/219dcb7` },
  },
  {
    date: '2026-11-03',
    title: 'First real-library import and checked XML export.',
    detail: 'A Rekordbox USB export loads into a new catalog and writes a new XML file.',
    source: { label: 'Library import', href: `${repo}/commit/40b0631` },
  },
];

/**
 * Cropped captures of the provisional prototype (synthetic audio, Tokyo Night,
 * 1440x900 Bot Screen). Provenance: public/images/README.md.
 */
export const screenshots = {
  deckMixerV2: {
    id: 'deck-mixer-v2',
    width: 1440,
    height: 285,
    halfWidth: 720,
    alt: 'Current deck row with two synthetic tracks, hot-cue pads and the compact mixer in the centre.',
    caption: 'Decks and compact mixer in layout B. Synthetic tracks; provisional application.',
  },
  layoutB: {
    id: 'layout-b',
    width: 1440,
    height: 635,
    halfWidth: 720,
    alt: 'Arrangement B: waveform above two decks with a mixer between them.',
    caption: 'B, default: waveforms above the decks and central mixer.',
  },
  layoutA: {
    id: 'layout-a',
    width: 1440,
    height: 628,
    halfWidth: 720,
    alt: 'Arrangement A: decks and central mixer above the waveform.',
    caption: 'A: decks and mixer above the waveforms.',
  },
  layoutC: {
    id: 'layout-c',
    width: 1440,
    height: 628,
    halfWidth: 720,
    alt: 'Arrangement C: waveform on top, decks below with the mixer on the right.',
    caption: 'C: waveforms above; mixer docked right.',
  },
  layoutD: {
    id: 'layout-d',
    width: 1440,
    height: 473,
    halfWidth: 720,
    alt: 'Arrangement D: mixer beside waveforms with decks below.',
    caption: 'D: mixer beside the waveforms; decks below.',
  },
  overview: {
    id: 'app-overview',
    width: 1440,
    height: 635,
    halfWidth: 720,
    alt: 'OxiDJ deck layout B with an orange and blue merged waveform, separate overviews, two synthetic tracks on the decks, and a compact mixer between them.',
    caption: 'Deck layout B in the provisional application, with synthetic tracks. Click to zoom.',
  },
  decksMerged: {
    id: 'decks-merged',
    width: 1440,
    height: 256,
    halfWidth: 720,
    alt: 'Merged two-deck waveform while both decks play: deck A at 120 BPM above the centre line in red-orange, amber and a pale yellow core, deck B at 124 BPM below in deep blue, cyan and an ice-white core, beat lines and a readout of the beat offset and BPM difference at the playhead.',
    caption: 'One merged waveform: deck A above, deck B below, three bands each.',
  },
  decksSynced: {
    id: 'decks-synced',
    width: 560,
    height: 256,
    alt: 'Centre of the merged 3-band waveform after SYNC while both decks play: KEY and SYNC lit on deck B, its tempo fader at minus 3.2 percent and the readout showing 0 ms and plus 0.00 BPM, with the beats of both decks lined up.',
    caption: 'SYNC with key lock on deck B: tempo and beats matched.',
  },
  syncRefused: {
    id: 'sync-refused',
    width: 720,
    height: 56,
    alt: 'Deck B control strip showing the message SYNC: deck B has no beat grid, next to the loaded synthetic file name.',
    caption: 'SYNC says why it cannot run.',
  },
  decksMixer: {
    id: 'decks-mixer',
    width: 1440,
    height: 120,
    halfWidth: 720,
    alt: 'Control strip with both decks playing above the optional mixer row: trim, high, mid and low EQ, filter and volume with a live level meter per deck, crossfader assign buttons, master volume and meter, and the crossfader in the centre.',
    caption: 'The optional mixer, hidden by default.',
  },
  bandsBefore: {
    id: 'bands-before',
    width: 1440,
    height: 422,
    halfWidth: 720,
    alt: 'Before: the deck area of the previous version, with deck A drawn almost only in orange and deck B almost only in blue, and flat cream and pale blue overview strips with small cue marks.',
    caption: 'Before: near-monochrome decks, flat overviews.',
  },
  bandsAfter: {
    id: 'bands-after',
    width: 1440,
    height: 422,
    halfWidth: 720,
    alt: 'After: the same deck area with low, mid and high bands nested in each deck colour, and overview strips in the same three bands with the played part dimmed, numbered cue flags and the visible window marked.',
    caption: 'After: three bands in detail and overviews.',
  },
  gridFallback: {
    id: 'grid-fallback',
    width: 1440,
    height: 392,
    halfWidth: 720,
    alt: 'Deck A shows a waveform without a beat grid over a minutes and seconds time grid; deck B shows only the time grid with the note analysing, waveform not ready.',
    caption: 'No beat grid or no waveform yet: a time grid instead.',
  },
  decksCollapsed: {
    id: 'decks-collapsed',
    width: 1440,
    height: 300,
    halfWidth: 720,
    alt: 'Deck area with the detail waveform collapsed: the control strip and two overview strips remain, and the track list moves up to use the space.',
    caption: 'Collapse any section; the library takes the space.',
  },
  waveformGrid: {
    id: 'waveform-beat-grid',
    width: 359,
    height: 438,
    alt: 'Mono waveform of a synthetic house loop with gain sliders, beat lines and Bar 6 to Bar 9 lines, an overview strip, and the caption beat grid: computed (taktus 0.1.0), 127.95 BPM.',
    caption: 'Computed beat grid on a synthetic 128 BPM loop.',
  },
  waveformClick: {
    id: 'waveform-click-grid',
    width: 359,
    height: 438,
    alt: 'Waveform of a synthetic click track where every click sits on a computed beat line, Bar 4 to Bar 7, with the caption beat grid: computed (taktus 0.1.0), 120.00 BPM.',
    caption: 'A synthetic 120 BPM click track: each click on a beat line.',
  },
  importedPlaylists: {
    id: 'imported-playlist-load-controls',
    width: 175,
    height: 252,
    alt: 'Actual OxiDJ library window: the synthetic imported playlist tree has Folder expanded and Inner First selected; its stored-order member 1 · Song C is selected, with explicit Load A and Load B buttons below.',
    caption:
      'Synthetic imported playlist: select a member, then explicitly load it to A or B. No autoplay.',
  },
} as const satisfies Record<string, Screenshot>;
