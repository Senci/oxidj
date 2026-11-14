# Dependency update inventory (work in progress)

Registry lookups: 2026-11-14. This is not a completion or platform-qualification claim. Rust versions below are prior direct requirements / latest stable observed; path dependencies excluded. Different Cargo graphs may resolve different versions.

| Dependency | Before | Latest observed | Current work |
|---|---|---|---|
| claxon | 0.4.3 | 0.4.3 | unchanged |
| cpal | 0.18.2 | 0.18.2 | unchanged |
| eframe | 0.32.3 | 0.37.0 | probe migrated; library UI remains 0.32.3: 0.37 produces extensive API and test compile errors |
| hound | 3.5.1 | 3.5.1 | unchanged |
| image | 0.25.8 | 0.25.10 | updated |
| lofty | 0.25.5 | 0.25.5 | unchanged |
| nanomp3 | 0.2.0 | 0.2.0 | unchanged |
| quick-xml | 0.41.0 | 0.42.0 | attempted; byte-oriented XML APIs and normalisation changed, compiler rejects existing code; reverted pending safe migration |
| raw-window-handle | 0.6.2 | 0.6.2 | unchanged |
| rtrb | 0.4.1 | 0.4.1 | unchanged |
| rusqlite | 0.40.2 | 0.42.0 | root 0.41.1 due to migration 2.7 requiring 0.41; reader 0.40.2: 0.42 pulls sqlcipher-src with `blessing AND WTFPL`, rejected by existing license gate |
| rusqlite_migration | 2.6.0 | 2.7.0 | updated |
| serde | 1.0.229 | 1.0.229 | unchanged |
| serde_json | 1.0.154 | 1.0.154 | unchanged |
| sha2 | 0.10.9 | 0.11.1 | library updated; xtask and UI test code uses removed LowerHex support and remains on 0.10.9 pending adaptation |
| signalsmith-stretch | 0.1.3 | 0.1.3 | unchanged |
| taktus | 0.1.0 | 0.1.0 | unchanged |
| wayland-client | 0.31.15 | 0.31.15 | unchanged |
| wayland-protocols | 0.32.13 | 0.32.13 | unchanged |
| zeroize | 1.8.2 | 1.9.1 | updated |
| rekordbox-pdb git | b9896c4 | upstream HEAD b9896c4 | already current HEAD; latest tag v0.1.0 is older |

Site direct packages before/observed latest: @eslint/js 9.39.5/10.0.1; @types/node 22.20.5/26.6.6; @vitest/coverage-v8 3.2.7/5.1.0; eslint 9.39.5/10.15.0; eslint-config-prettier 9.1.2/10.1.8; happy-dom 21.0.4/21.0.4; prettier 3.9.10/3.10.1; typescript 5.9.3/7.0.2; typescript-eslint 8.71.2/8.72.2; vite 6.4.4/8.3.8; vitest 3.2.7/5.1.0. Updated to observed latest except TypeScript 6.0.3: 7.0.2 makes lint fail and typescript-eslint 8.72.2 declares peer `>=4.8.4 <6.1.0`.

The probe's eframe 0.37 dependency graph fails the repository's license policy when explicitly checked: `clipboard-win`/`error-code` BSL-1.0, default fonts OFL-1.1 and Ubuntu-font-1.0; the prototype itself lacks a declared license. Do not waive policy or install this work pending resolution and review.

Wayland: upstream [egui PR #8660](https://github.com/emilk/egui/pull/8660) describes committing suspended-window configures for glow and wgpu, and depends on winit support. The locally downloaded eframe 0.37.0/winit 0.30.13 sources do not contain `notify_presentation_paused`; upstream inclusion is **not demonstrated** and runtime behavior is **not tested**. Separately, [issue #8776](https://github.com/emilk/egui/issues/8776) reports hidden viewport screenshots still do not paint on 0.37.0. Do not present either as fixed.
