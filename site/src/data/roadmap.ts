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
  verifiedAt: '2026-10-31',
  currentFocus: 'M4: Rekordbox XML groundwork',
  currentSummary:
    'Synthetic Rekordbox XML fixtures support read-only preview, bounded export of a supported subset to a new file, and exact-value guarded edits.',
  working:
    'A persistent local catalog, reference-only WAV/FLAC/MP3 import and a provisional viewer with search, sort and track details work today.',
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
      href: `${repo}/blob/master/plan/STATUS.md#m0-09-verification-summary-independent-lead-run`,
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
      'Synthetic Rekordbox XML fixtures support read-only preview, bounded export of a supported subset to a new file, and guarded edits that check the expected old value. Unsupported content can prevent export. Compatibility with real Rekordbox or OneLibrary libraries has not been demonstrated.',
    source: { label: 'XML adapter and tests', href: `${repo}/tree/master/crates/oxidj-rekordbox` },
  },
  {
    id: 'm5',
    title: 'DJ metadata',
    status: 'planned',
    statusLabel: 'Later',
    detail: 'Cues, loops, beat grids and key.',
    source: { label: 'Priorities', href: `${repo}/blob/master/idea.md#priorities` },
  },
  {
    id: 'm6',
    title: 'Analysis',
    status: 'planned',
    statusLabel: 'Later',
    detail: 'Musical and technical analysis workflows.',
    source: { label: 'Priorities', href: `${repo}/blob/master/idea.md#priorities` },
  },
  {
    id: 'm7',
    title: 'Interoperability proof',
    status: 'planned',
    statusLabel: 'Later',
    detail: 'Import, manage and export a real library with supported metadata intact.',
    source: { label: 'MVP objective', href: `${repo}/blob/master/idea.md#mvp` },
  },
];
export const projectUpdates: readonly ProjectUpdate[] = [
  {
    date: '2026-10-29',
    title: 'No-edit export retains cue colour fields.',
    detail: 'A synthetic XML regression checks RGB fields survive a bounded export unchanged.',
    source: { label: 'Export change', href: `${repo}/commit/30a2d29` },
  },
  {
    date: '2026-10-27',
    title: 'Added an exact-value guarded Track Kind edit.',
    detail: 'The synthetic XML edit targets one track and writes to a new file.',
    source: { label: 'Guarded edit', href: `${repo}/commit/9b62674` },
  },
  {
    date: '2026-10-26',
    title: 'Export checks declared playlist counts.',
    detail: 'Inconsistent counts are rejected before a new XML file is written.',
    source: { label: 'Count validation', href: `${repo}/commit/3a961b9` },
  },
];
