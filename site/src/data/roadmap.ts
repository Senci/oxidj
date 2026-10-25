export interface SourceLink {
  label: string;
  href: string;
}

export interface Milestone {
  id: string;
  title: string;
  status: 'complete' | 'in-progress' | 'planned' | 'deferred';
  statusLabel: string;
  goal: string;
  progress: string;
  next: string;
  source: SourceLink;
}

export interface ProjectUpdate {
  date: string;
  category: string;
  title: string;
  detail: string;
  source: SourceLink;
}

const repo = 'https://github.com/Senci/oxidj';

export const roadmapSnapshot = {
  verifiedAt: '2026-10-25',
  currentSummary:
    'The local catalog and importer are taking shape. The integrated library UI is the active build; the isolated 200k-row interaction probe is reviewed.',
  currentFocus: 'Persisted catalog UI and real-app smoke',
  qualificationSummary:
    'The 500k-row / 60fps target is preserved for a later qualification workstream. It is deferred and unverified—not passed, and not a gate for the bounded M1–M3 local slice.',
  qualificationSource: `${repo}/blob/master/plan/MVP.md#qualification-checkpoint`,
  repo,
} as const;

export const milestones: readonly Milestone[] = [
  {
    id: 'M0',
    title: 'Foundation',
    status: 'complete',
    statusLabel: 'Verified complete',
    goal: 'Set the Rust workspace, architecture, repository boundaries and quality gates.',
    progress: 'M0 was independently verified on 2026-10-15.',
    next: 'Foundation complete; local-library delivery is underway.',
    source: { label: 'M0 status', href: `${repo}/blob/master/plan/STATUS.md` },
  },
  {
    id: 'M1',
    title: 'Persistent local catalog',
    status: 'in-progress',
    statusLabel: 'In progress',
    goal: 'Store stable track identities and technical facts in a bounded, local SQLite catalog.',
    progress:
      'Catalog, restart, backup and restore work is present. Full M1 acceptance and release evidence remain open.',
    next: 'Close remaining migration, failure-handling and end-to-end acceptance evidence.',
    source: { label: 'M1 acceptance', href: `${repo}/blob/master/plan/MVP.md` },
  },
  {
    id: 'M2',
    title: 'Reference-only file import',
    status: 'in-progress',
    statusLabel: 'In progress',
    goal: 'Import local WAV, MP3 and FLAC references without moving or modifying source audio.',
    progress:
      'Importer and format-specific code is in the library. Full matrix, error-path and source-immutability evidence remains part of acceptance.',
    next: 'Finish importer acceptance on synthetic fixtures and record exact outcomes.',
    source: { label: 'M2 acceptance', href: `${repo}/blob/master/plan/MVP.md` },
  },
  {
    id: 'M3',
    title: 'Usable library UI',
    status: 'in-progress',
    statusLabel: 'Active build',
    goal: 'Browse and search the persistent catalog through shared application handlers.',
    progress:
      'Integrated UI work is active. The isolated egui probe is reviewed: Bot Screen showed the insertion line, row reorder and corrected keyboard scrolling.',
    next: 'Complete the persisted-catalog UI and import/reopen interaction smoke.',
    source: { label: 'M3 acceptance', href: `${repo}/blob/master/plan/MVP.md` },
  },
  {
    id: 'M4',
    title: 'OneLibrary / Rekordbox interoperability',
    status: 'planned',
    statusLabel: 'Separate workstream',
    goal: 'Preserve meaning across vendor-library import and export.',
    progress: 'Kept separate from bounded local M1–M3 delivery.',
    next: 'Start after the local-library contract; use reviewed schemas and round-trip fixtures.',
    source: { label: 'Project plan', href: `${repo}/blob/master/plan/MVP.md` },
  },
  {
    id: 'M5',
    title: 'DJ metadata',
    status: 'planned',
    statusLabel: 'Planned',
    goal: 'Add editable DJ metadata such as cues, loops, beat grids and key.',
    progress: 'Not part of the current local-library slice.',
    next: 'Define metadata contracts and safe round-trip behavior after interoperability groundwork.',
    source: { label: 'Product vision', href: `${repo}/blob/master/idea.md` },
  },
  {
    id: 'M6',
    title: 'Analysis',
    status: 'planned',
    statusLabel: 'Planned',
    goal: 'Add analysis workflows for musical and technical track properties.',
    progress: 'Deferred until the library and metadata foundations are dependable.',
    next: 'Plan analysis data, provenance and recomputation behavior.',
    source: { label: 'Product vision', href: `${repo}/blob/master/idea.md` },
  },
  {
    id: 'M7',
    title: 'Interoperability proof',
    status: 'planned',
    statusLabel: 'Planned',
    goal: 'Prove library exchange keeps supported vendor data intact.',
    progress: 'Longer-term product proof; not current M1–M3 acceptance.',
    next: 'Build independently reviewed fixtures and end-to-end round-trip evidence.',
    source: { label: 'Project plan', href: `${repo}/blob/master/plan/MVP.md` },
  },
];

export const projectUpdates: readonly ProjectUpdate[] = [
  {
    date: '2026-10-25',
    category: 'M3 · interaction',
    title: 'Keyboard focus now stays in context.',
    detail:
      'The 200k-row probe keeps the viewport stable while focus stays visible, then scrolls only enough to reveal focus at either edge. Independent review approved the fix.',
    source: { label: 'Fix e65c6f6', href: `${repo}/commit/e65c6f6` },
  },
  {
    date: '2026-10-25',
    category: 'M3 · prototype',
    title: 'Drag target is visible before release.',
    detail:
      'Bot Screen evidence shows the insertion line between rows and confirms the resulting reorder after release. This is isolated-probe evidence, not a production or performance qualification.',
    source: {
      label: 'Probe evidence',
      href: `${repo}/tree/master/prototypes/oxidj-gui-probe/evidence/current-bot-screen`,
    },
  },
  {
    date: '2026-10-23',
    category: 'M1–M3 · scope',
    title: 'Local-library acceptance was separated from later work.',
    detail:
      'The bounded local slice is reference-only and capped at 10,000 tracks. Vendor interoperability and 500k qualification remain separate.',
    source: { label: 'MVP contract', href: `${repo}/blob/master/plan/MVP.md` },
  },
  {
    date: '2026-10-15',
    category: 'M0 · foundation',
    title: 'Foundation verified.',
    detail:
      'The Rust workspace, architecture, tooling and repo-owned quality gates were independently verified.',
    source: { label: 'Project status', href: `${repo}/blob/master/plan/STATUS.md` },
  },
];

export const productGoals = [
  'Trustworthy local DJ library management',
  'Strong OneLibrary and Rekordbox interoperability',
  'Excellent cross-platform desktop UX',
  'DJ metadata and analysis after the library core',
  'Realtime DJ functionality later',
] as const;
