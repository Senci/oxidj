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
  verifiedAt: '2026-10-26',
  currentSummary:
    'Bounded local M1–M3 acceptance is complete. M4 is in progress with synthetic XML fixtures, read-only preview and bounded subset export implemented; full vendor compatibility remains unpassed.',
  currentFocus: 'M4: bounded Rekordbox XML groundwork',
  qualificationSummary:
    'The 500k-row / 60fps target is preserved for a later qualification workstream. It is deferred and unverified—not passed, and not a gate for the bounded M1–M3 local slice.',
  qualificationSource: `${repo}/blob/master/plan/MVP.md`,
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
    status: 'complete',
    statusLabel: 'Acceptance complete',
    goal: 'Store stable track identities and technical facts in a bounded, local SQLite catalog.',
    progress:
      'Independent review approved stable identities/facts, transactions and retries, migration rollback, backup/restore, and fresh-process fact reopen.',
    next: 'M1 closed within the 10,000-track local scope; 500k qualification remains separate and unpassed.',
    source: { label: 'M1 acceptance', href: `${repo}/blob/master/plan/MVP.md` },
  },
  {
    id: 'M2',
    title: 'Reference-only file import',
    status: 'complete',
    statusLabel: 'Acceptance complete',
    goal: 'Import local WAV, MP3 and FLAC references without moving or modifying source audio.',
    progress:
      'Independent review approved declared WAV/MP3/FLAC outcomes, failure/cancellation cases, source immutability, and an owner-authorized local MP3 smoke.',
    next: 'M2 closed for reference-only import; embedded tags and Rekordbox/OneLibrary exchange remain out of scope.',
    source: { label: 'M2 acceptance', href: `${repo}/blob/master/plan/MVP.md` },
  },
  {
    id: 'M3',
    title: 'Usable library UI',
    status: 'complete',
    statusLabel: 'Provisional smoke complete',
    goal: 'Browse and search the persistent catalog through shared application handlers.',
    progress:
      'Independent review approved synthetic import, fresh-GUI reopen without rescan, search, sort, selection/details, and empty/error/missing states. Evidence is in the standalone prototype report.',
    next: 'Production framework admission and cross-platform accessibility remain separate, unpassed gates.',
    source: { label: 'M3 acceptance', href: `${repo}/blob/master/plan/MVP.md` },
  },
  {
    id: 'M4',
    title: 'OneLibrary / Rekordbox interoperability',
    status: 'in-progress',
    statusLabel: 'In progress',
    goal: 'Preserve meaning across vendor-library import and export.',
    progress:
      'A1 synthetic XML fixtures, read-only Rekordbox XML preview, and bounded fail-closed, no-edit subset export are implemented. This is not full Rekordbox app or OneLibrary compatibility.',
    next: 'Full Rekordbox app/OneLibrary compatibility and ADR-005 promotion remain unpassed; A1–A3/C1–C6 qualification remains OPEN / NOT QUALIFYING. Production GUI admission and 500k/60fps gates remain separate.',
    source: { label: 'M4 status and evidence', href: `${repo}/blob/master/plan/STATUS.md` },
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
    date: '2026-10-26',
    category: 'M4 · XML groundwork',
    title: 'Bounded Rekordbox XML paths implemented.',
    detail:
      'A1 synthetic fixtures, read-only preview and fail-closed, no-edit subset export are implemented. Full Rekordbox app/OneLibrary compatibility, ADR-005 promotion and A1–A3/C1–C6 qualification remain unpassed.',
    source: { label: 'M4 status and evidence', href: `${repo}/blob/master/plan/STATUS.md` },
  },
  {
    date: '2026-10-26',
    category: 'M1–M3 · acceptance',
    title: 'Bounded local-library acceptance closed.',
    detail:
      'Independent reviews approved M1 catalog recovery, M2 reference-only import and private-sample smoke, and provisional M3 persisted-GUI smoke. Local `cargo xtask ci-full` and exact-head GitHub CI passed; production GUI, vendor interoperability, and 500k/60fps remain unpassed.',
    source: { label: 'Acceptance closeout', href: `${repo}/blob/master/plan/MVP.md` },
  },
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
