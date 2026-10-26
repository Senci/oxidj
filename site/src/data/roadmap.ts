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
  verifiedAt: '2026-10-26',
  currentFocus: 'M4: Rekordbox XML groundwork',
  currentSummary:
    'Synthetic XML fixtures, read-only preview and limited, unedited subset export are implemented.',
  working:
    'A persistent local catalog, reference-only WAV/FLAC/MP3 import and a provisional viewer with search, sort and track details work today.',
  unfinished:
    'Production GUI, accessibility and platform validation, full Rekordbox/OneLibrary compatibility, and 500k-track / 60fps performance are unfinished or unverified.',
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
      'Start with documented Rekordbox XML: synthetic fixtures, read-only preview and export to a new file for a limited, unedited subset. Unsupported content prevents export. This work is separate from the local catalog.',
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
    date: '2026-10-26',
    title: 'XML export refuses inconsistent playlist counts.',
    detail: 'Track relationships and declared counts are checked before a new file is written.',
    source: { label: 'Export fix', href: `${repo}/commit/3a961b9` },
  },
  {
    date: '2026-10-26',
    title: 'Catalog and importer reviews completed.',
    detail: 'Fresh-process reopen and unchanged-source checks joined the local acceptance record.',
    source: { label: 'Acceptance record', href: closeout },
  },
  {
    date: '2026-10-25',
    title: 'Keyboard focus stays in view.',
    detail: 'The separate interaction prototype scrolls only when focus reaches the visible edge.',
    source: { label: 'Interaction fix', href: `${repo}/commit/e65c6f6` },
  },
];
