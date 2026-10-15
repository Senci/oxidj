# M0-06: GUI Framework Evaluation

**Status:** Research complete — review ready
**Owner:** oxidj-2
**Date:** 2026-10-15

---

## Executive Summary

OxiDJ needs a cross-platform Rust desktop GUI framework for M3 (Usable library UI) and beyond. This evaluation covers the 2025-2026 landscape with primary sources, focusing on:

- **Virtualized lists** for 100k+ track libraries
- **Accessibility (a11y)** via AccessKit integration
- **Testing** story (headless, platform smoke tests)
- **Platform support** (Linux/macOS/Windows)
- **Maintenance risk** (funding, contributor continuity)

**No decision without scoped Sol review.**

---

## Framework Landscape (2025-2026)

### Top Contenders for OxiDJ

| Framework | Paradigm | A11y | Virtualization | IME | Platform | License | Notes |
|-----------|----------|------|----------------|-----|----------|---------|-------|
| **Slint** | Declarative DSL | ✅ AccessKit (default) | ✅ Built-in `ListView` | ✅ winit + Android | Linux/macOS/Win + embedded | GPL-3.0 / commercial | Best "traditional" desktop UX; design tool |
| **Tauri 2** | WebView (HTML/JS) | ✅ Native (via web) | ✅ Browser virtualization | ✅ Browser | Linux/macOS/Win | MIT/Apache-2.0 | Web tech team; system webview (no bundle) |
| **egui/eframe** | Immediate | ✅ AccessKit (mandatory 0.35+) | ✅ Natural (immediate) | ✅ winit (0.35 preedit) | Linux/macOS/Win/Web/WASM | MIT/Apache-2.0 | Tools/debug UIs; accessibility gaps documented |
| **iced** | Retained (Elm) | ❌ No stable AccessKit | ❌ No built-in | ✅ 0.14+ | Linux/macOS/Win | MIT | Strong MVU testing; a11y blocked 6+ years |
| **GPUI** | Hybrid | ✅ AccessKit (May 2026, early) | Unknown | In-house | macOS/Win/Linux | Apache-2.0 (app GPL) | Zed-driven; custom windowing; no releases 8+ months |
| **Xilem/Linebender** | Reactive view-tree | ✅ AccessKit (non-optional) | Unknown | winit | Linux/macOS/Win | Apache-2.0 | Research-grade; zero production users |

**Sources:** [rust-gui-desktop-ecosystem-state](https://github.com/GoldStrikeArch/rust-gui-desktop-ecosystem-state) (snapshot 2026-07-07, reconciled through 2026-07-10) — stack-rows.md, 00-ecosystem-map.md, 10-empirical-results.md; [boringcactus 2025 survey](https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html); [Rust SME Cookbook](https://rust.codeguides.io/desktop-gui/).

---

## Deep Dive: Virtualized Lists for Large Libraries

### OxiDJ Requirement
- 100k–500k tracks in library view
- Smooth 60fps scrolling
- Search/filter without full re-render

### Framework Capabilities

**egui (immediate mode)** — Natural virtualization: only visible widgets exist each frame. Reddit user reports "100K records without any visible performance delay" ([r/rust](https://www.reddit.com/r/rust/comments/1cth129/)). Use `ui.push_id` for stable widget IDs in dynamic lists.

**Slint** — Built-in `ListView` with virtualization; empirical test: "dark mode + Enter-to-submit + list virtualization came free from built-ins" (stack-rows.md). 42 Rust LoC + 52 DSL for Tasks spec.

**Tauri** — Delegates to browser engine (WebView2/WKWebView/WebKitGTK). Browser virtualization is mature; "performs quite well" at 100k rows (r/rust).

**iced** — **No built-in virtualization**; 11k lines → +313 MB RSS in ecosystem benchmark (goldstrikearch.github.io). "Frames dropped, entire UI feels sluggish" at 100k rows (r/rust). Virtualization requires custom implementation.

**GPUI/Xilem** — No public benchmarks for large lists.

### Recommendation
For dense track lists: **egui** (simplest mental model), **Slint** (declarative + built-in), or **Tauri** (if web tech available). Avoid iced without custom virtualization work.

---

## Deep Dive: Accessibility (a11y)

### AccessKit: The Ecosystem Convergence Point

AccessKit is the **de-facto shared a11y layer** across Rust GUI:

| Framework | AccessKit Status | Evidence |
|-----------|------------------|----------|
| **egui/eframe** | Mandatory dep since 0.35; schema types required by core | PR #2294, eframe Cargo.toml |
| **Slint** | Default feature; built-in | stack-rows.md |
| **Xilem** | Non-optional; parley accesskit feature | stack-rows.md |
| **GPUI** | Merged 2026-05-27; early stage | stack-rows.md |
| **Tauri/Dioxus** | Native via webview HTML semantics | stack-rows.md |
| **iced** | **Missing in stable** — issue #552 (2020); PR #3111 open-unmerged; hecrj closed #3281 "I'll work on this myself" | stack-rows.md, boringcactus |

**Gaps documented in egui** (accesskit 0.24+): text-input widgets incompletely exposed (caret/braille #2895/#8732), RTL/BiDi essentially unsupported (#1317). IME works including macOS CJK. (stack-rows.md)

**Linux AT-SPI risk:** Wayland-native successor "Newton" is still a prototype (GNOME STF report 2025-04-11).

### OxiDJ Implication
If a11y is a hard requirement (public DJ app), **iced is disqualified**. egui has gaps but is usable with stakeholder documentation. Slint, Tauri, GPUI, Xilem have working AccessKit integration.

---

## Deep Dive: Testing Story

### Ecosystem Guidance (Rust SME Cookbook)
> "Test pure state transitions and use cases without a GUI runtime; add focused framework, accessibility, and platform smoke tests only where they prove behavior the lower layer cannot."

### Framework Testing Capabilities

| Framework | Headless Testing | A11y Testing | Platform Smoke |
|-----------|------------------|--------------|----------------|
| **iced** | ✅ Pure `update` functions unit test without window | ❌ N/A | Limited |
| **egui/eframe** | ⚠️ Frame-bound; `kittest` built on AccessKit tree | ✅ `kittest` | eframe `App` callback |
| **Slint** | ✅ Headless mode; component testing via generated bindings | ✅ AccessKit tree | Event-loop bridge |
| **Tauri** | ⚠️ WebView required; IPC commands testable | Via webview | WebView + Rust commands |
| **Xilem** | ✅ View-tree diffing | ✅ AccessKit | winit/masonry |

**Source:** [Rust Desktop GUI Skill](https://github.com/nledford/engineering-review-board/blob/main/skills/rust-desktop-gui/SKILL.md), [Rust SME Cookbook](https://rust.codeguides.io/desktop-gui/best-practices/).

---

## Maintenance & Continuity Risk

### Key Risk Signals (2025-2026)

| Risk | Impact | Evidence |
|------|--------|----------|
| **notgull (winit/softbuffer) burnout hiatus** | Windowing layer continuity | [notgull.net/burnout](https://notgull.net/burnout/) March 2025 |
| **Raph Levien (Linebender) left Google Oct 2025 → Canva** | Parley/Vello/xilem funding transition | Linebender TMIL-19 |
| **GNOME STF a11y grant wrapped up** | AccessKit funding | [GNOME STF report](https://blogs.gnome.org/tbernard/2025/04/11/gnome-stf-2024/) |
| **NLnet NGI0 grants for Parley (2026)** | Positive: Parley funded through 2026 | [nlnet.nl/project/Parley](https://nlnet.nl/project/Parley/) |
| **Tauri governance** | Most formal (Commons Conservancy, elected board) but core maintenance coupled to CrabNebula | stack-rows.md |
| **Slint commercial backing** | Sustainable but GPL/commercial model | slint.dev |
| **GPUI** | Zed-driven; 0 releases in 8.5 months; main diverged hugely | stack-rows.md |

### Coordination
RustWeek 2025/2026 UI summits: "various project leaders increasingly interested in pooling resources... but still a long way to go" (Linebender TMIL-17).

---

## OxiDJ-Specific Recommendations

### For M3 (Library UI) — Short Term
1. **Prototype in egui/eframe** — fastest iteration for tools; immediate mode suits "dense virtualized" requirement naturally; AccessKit mandatory since 0.35.
2. **Evaluate Slint** — if design-tool workflow matters; declarative DSL separates UI from logic; built-in virtualization.
3. **Defer Tauri** — unless team has web expertise; adds IPC complexity.

### For M3+ — Decision Gates
- **Accessibility review** with real screen readers (NVDA, VoiceOver, Orca) on all three platforms
- **Virtualization benchmark** with 200k synthetic tracks
- **Packaging test** (AppImage, .dmg, .msi/.exe) on CI
- **Sol review** before commitment

### Open Risks
- Linux Wayland fractional scaling + NVIDIA (WebKitGTK blank-window bugs affect Tauri)
- iced a11y timeline uncertain (6+ years)
- GPUI release cadence stalled
- Xilem zero production users

---

## Evidence Packets for Review

Each framework should have a review packet with:
- [ ] Source citations (this doc)
- [ ] Current version + release date
- [ ] AccessKit integration status + gaps
- [ ] Virtualization approach + benchmark (if any)
- [ ] Platform matrix (Linux/macOS/Win) + CI status
- [ ] License + commercial implications
- [ ] Maintenance health (contributors, funding, issue velocity)
- [ ] Prototype repo + build/test commands

---

## References

1. GoldStrikeArch, *rust-gui-desktop-ecosystem-state* (2026-07-07 snapshot) — [GitHub](https://github.com/GoldStrikeArch/rust-gui-desktop-ecosystem-state)
2. boringcactus, *A 2025 Survey of Rust GUI Libraries* (2025-04-13) — [blog](https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html)
3. Rust SME Cookbook, *Desktop GUI* — [codeguides.io](https://rust.codeguides.io/desktop-gui/)
4. nledford, *Rust Desktop GUI Skill* — [GitHub](https://github.com/nledford/engineering-review-board/blob/main/skills/rust-desktop-gui/SKILL.md)
5. goldstrikearch.github.io, *The Rust desktop GUI ecosystem, measured* — [site](https://goldstrikearch.github.io/rust-gui-desktop-ecosystem-state/)
6. Reddit r/rust, *100k rows shouldn't make the UI laggy* (2025) — [thread](https://www.reddit.com/r/rust/comments/1cth129/)