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
  verifiedAt: '2026-11-05',
  currentFocus: 'M8: two decks, playback and mixer, then M4 and M7 real-library exchange',
  currentSummary:
    'The library window now has two playable decks: one merged waveform with deck A above and deck B below, an audio engine, hot cues and loops, tempo with key lock, SYNC and an optional mixer. Listening and DJ hardware checks are still open. On the exchange side, synthetic Rekordbox XML fixtures support read-only preview, bounded export of a supported subset to a new file, and exact-value guarded edits. On one real library, playlists, track details, beat grids and cues went into a catalog and a new XML file, and the owner confirmed in Rekordbox on Windows that the export and an edited re-export load with tracks, BPM, key, cues, beat grids, playlists and the edits intact.',
  working:
    'A persistent local catalog, reference-only WAV/FLAC/MP3/AIFF import and a provisional viewer with search, sort and track details work today. The library window has a full-width track list with side panels you can show or hide, and draws a smooth vector waveform (mono, 3-band or RGB) for the selected track with bar and beat lines from an imported beat grid, or from its own computed BPM and beat grid when none was imported. Tracks load onto two decks shown in one merged waveform (deck A warm, above; deck B cool, below) with a separate overview per deck. Every deck section can be collapsed, and Settings (Ctrl+,) keeps the choices. A CPAL audio engine plays both decks with cue, play, eight hot cues and loops; the tempo fader has key lock, SYNC matches tempo and beat phase, and an optional mixer adds trim, 3-band EQ, filter, faders, a crossfader and meters. A user-local install script adds a launcher and desktop menu entry on Linux. From the command line, a Rekordbox USB export can be read into a new catalog and written back out as a checked XML file, with Windows drive letters via --drive.',
  unfinished:
    'Beyond one owner-run round trip through Rekordbox on Windows, compatibility with real Rekordbox or OneLibrary libraries and DJ hardware (CDJ/XDJ) has not been demonstrated. Computed BPM is a suggestion, not Rekordbox parity. The decks have not had an owner listening check, have not been used with club gear or controllers, and make no latency guarantee: one 600 s run through PipeWire had 1 audio dropout (xrun), and moving the tempo fader with key lock on can shift the next beat by up to 5 ms. Production desktop, accessibility and platform testing is still pending, as is performance testing at 500,000 tracks and 60 frames per second.',
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
    status: 'planned',
    statusLabel: 'Later',
    detail:
      'Cues, loops, beat grids and key. Beat grids and cues from a real library are already read during import; editing them comes later.',
    source: { label: 'Priorities', href: `${repo}/blob/master/idea.md#priorities` },
  },
  {
    id: 'm6',
    title: 'Analysis',
    status: 'in-progress',
    statusLabel: 'First slice',
    detail:
      'Musical and technical analysis workflows. The first piece is a smooth vector waveform view with mono, 3-band and RGB styles, live per-band EQ gain, and bar and beat lines when the imported library has a beat grid. OxiDJ now also computes BPM and a beat grid itself (with the taktus library) and shows them when no imported grid exists; this is a suggestion, not Rekordbox parity: on 90 real tracks the BPM was within 2% of Rekordbox on 91%, beat-tracking score F70 0.76. Downbeats, tempo changes and key are not detected yet.',
    source: { label: 'Priorities', href: `${repo}/blob/master/idea.md#priorities` },
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
      'Two decks in the library window: one merged waveform (deck A warm, above the centre line; deck B cool, below) with a separate overview per deck, collapsible sections and Settings (Ctrl+,). A CPAL audio engine plays both decks with cue, play, eight hot cues and loops. Tempo has key lock (Signalsmith Stretch), SYNC matches tempo and beat phase and refuses visibly when a deck has no beat grid, and an optional mixer adds trim, 3-band EQ, filter, faders, a crossfader and meters; when hidden it passes audio through unchanged. Not yet demonstrated: an owner listening check, club gear and controllers, and any latency guarantee. One 600 s PipeWire run had 1 xrun; a key-locked tempo move can shift the next beat by up to 5 ms.',
    source: { label: 'Decks plan', href: `${repo}/blob/master/plan/MVP.md` },
  },
];
export const projectUpdates: readonly ProjectUpdate[] = [
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
  overview: {
    id: 'app-overview',
    width: 1440,
    height: 680,
    halfWidth: 720,
    alt: 'OxiDJ library window with both decks playing: a toolbar with search and panel toggles, one merged waveform with deck A in orange above the centre line and deck B in blue below, a control strip with cue, pause, eight hot cues, loops, tempo faders, SYNC and KEY per deck, a mixer row, two overview strips and a list of four synthetic tracks with A and B badges.',
    caption: 'Provisional prototype with synthetic audio. Click to zoom.',
  },
  decksMerged: {
    id: 'decks-merged',
    width: 1440,
    height: 392,
    halfWidth: 720,
    alt: 'Merged two-deck waveform: deck A at 120 BPM in orange above the centre line, deck B at 123.97 BPM in blue below, numbered hot cues, a readout of the beat offset and BPM difference at the playhead, the control strip and one overview strip per deck.',
    caption: 'Both decks loaded: one merged waveform, two overviews.',
  },
  decksSynced: {
    id: 'decks-synced',
    width: 560,
    height: 256,
    alt: 'Centre of the merged waveform after SYNC: KEY and SYNC lit on deck B, its tempo fader at minus 3.2 percent and the readout showing plus 0 ms and plus 0.00 BPM, with the beats of both decks lined up.',
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
    alt: 'Control strip above the optional mixer row: trim, high, mid and low EQ, filter and volume with a level meter per deck, crossfader assign buttons, master volume and meter, and the crossfader in the centre.',
    caption: 'The optional mixer, hidden by default.',
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
} as const satisfies Record<string, Screenshot>;
