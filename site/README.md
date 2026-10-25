# OxiDJ project log

Repository-backed GitDocs site for OxiDJ's stages, goals, current progress and roadmap.
The homepage is the milestone timeline. The previous FILE #0042 dossier remains at
`/dossier.html`.

## Keep it current

Update `src/data/roadmap.ts` when a milestone changes, a goal is accepted, or a reviewed
result changes project status. Set `roadmapSnapshot.verifiedAt` to the date the Kanban board
and canonical plan were checked. Keep `plan/MVP.md`, `plan/STATUS.md` and ADRs as the source
of truth; the site is a readable snapshot, not a second decision record. Link to evidence,
and label deferred or unverified gates as such.

Current status was read from the OxiDJ Kanban board and repository plan on 2026-10-25.
M1–M3 delivery is in progress; M0 is verified complete. The active local UI work is not a
production framework approval. The 500k-row / 60fps qualification remains deferred and
unverified.

## Local workflow

Use Bun. Commands run locally and do not publish the site.

```bash
bun install
bun run dev
bun run typecheck
bun run lint
bun run build
bun run preview
```

The static build is written to `dist/`. Vite uses a relative base path for repository
subpaths; both the roadmap and dossier are included in the build.

## Structure

```text
site/
├── index.html                 # project timeline homepage
├── dossier.html               # retained FILE #0042 teaser
├── src/data/roadmap.ts        # structured, dated roadmap snapshot
├── src/roadmap.ts             # roadmap renderer
├── src/styles/roadmap.css     # timeline UI
└── src/                       # original dossier app and tests
```

## Licence

MIT. See `LICENSE`.
