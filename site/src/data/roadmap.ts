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
export interface ProjectUpdate {
  date: string;
  title: string;
  detail: string;
  source: SourceLink;
}
const repo = 'https://github.com/Senci/oxidj';
const closeout = `${repo}/blob/master/plan/MVP.md#bounded-local-m1m3-acceptance-closeout-2026-10-26`;
export const roadmapSnapshot = {
  verifiedAt: '2026-11-03',
  currentFocus: 'M4 and M7: first real-library round trip',
  currentSummary:
    'Synthetic Rekordbox XML fixtures support read-only preview, bounded export of a supported subset to a new file, and exact-value guarded edits. A first run on one real library export carried playlists, track details, beat grids and cues into a catalog and a new XML file.',
  working:
    'A persistent local catalog, reference-only WAV/FLAC/MP3 import and a provisional viewer with search, sort and track details work today. The library window draws a computed waveform for the selected track. From the command line, a Rekordbox USB export can be read into a new catalog and written back out as a checked XML file.',
  unfinished:
    'Compatibility with real Rekordbox or OneLibrary libraries has not been demonstrated. Production desktop, accessibility and platform testing is still pending, as is performance testing at 500,000 tracks and 60 frames per second.',
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
      'Recursively import supported WAV, FLAC and MP3 encodings by reference. Originals stay in place and unchanged; filenames and technical facts enter the catalog.',
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
      'Synthetic Rekordbox XML fixtures support read-only preview, bounded export of a supported subset to a new file, and guarded edits that check the expected old value. Unsupported content can prevent export. A first import of a real library export, with playlists, track details, beat grids and cues, has been checked against an independent decoder. Compatibility with real Rekordbox or OneLibrary libraries has not been demonstrated.',
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
      'Musical and technical analysis workflows. The first piece is a waveform view with mono, 3-band and RGB styles and live per-band EQ gain. Tempo and beat-grid detection come next.',
    source: { label: 'Priorities', href: `${repo}/blob/master/idea.md#priorities` },
  },
  {
    id: 'm7',
    title: 'Interoperability proof',
    status: 'in-progress',
    statusLabel: 'First slices',
    detail:
      'Import, manage and export a real library with supported metadata intact. The first import and checked XML export has run on one private library. Rekordbox itself has not yet accepted an exported file.',
    source: { label: 'MVP objective', href: `${repo}/blob/master/idea.md#mvp` },
  },
];
export const projectUpdates: readonly ProjectUpdate[] = [
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
