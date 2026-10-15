---
id: 001
title: Select GUI framework via prototype-first evaluation
status: Provisional
review_status: completed 2026-10-15 (gpt-6.1-sol, reasoning low) — egui prototype confirmed, Slint compared, Qt deferred; confidence medium
review_confidence: medium
date: 2026-10-15
owners: [oxidj-2]
sources: [docs/research/gui-evaluation.md, docs/research/soundcraft.md]
sol_review: required
provisional_reason: Required independent Sol/high review not yet completed; suspended under the Nous-only free-model routing policy. Final framework commitment is additionally gated on the 200k-row virtualization + a11y prototype.
---

# ADR-001: GUI Framework

> **Provisional (M0-07).** Evidence-backed recommendation, not yet accepted architecture. The required independent Sol/high review has not been performed and is suspended under the Nous-only free-model routing policy (see M0-07 comment, 2026-10-15). The final framework commitment is additionally gated on a 200k-row virtualized-list + AccessKit smoke-test prototype. Do not add a GUI dependency to the M0 workspace until the decision transitions to Accepted.

## Context

OxiDJ needs a cross-platform Rust desktop GUI for M3 (Usable library UI) onward. The library view must render 100k–500k tracks with smooth scrolling. Accessibility (AccessKit) is a hard requirement for a public DJ app. This decision selects the prototype direction for M3; the final framework commitment is gated on prototype results.

## Decision

Adopt a **prototype-first, evaluate-second** path:

1. **Prototype candidate one:** `egui` / `eframe` — immediate mode gives natural virtualization for dense track lists with the least implementation effort. AccessKit is mandatory since 0.35.
2. **Evaluate candidate two:** `Slint` — declarative DSL with built-in `ListView` virtualization and AccessKit by default; evaluate if design-tool workflow matters.
3. **Defer:** Tauri 2 (unless a web-tech team is available; adds IPC complexity), iced (no stable AccessKit — disqualified if a11y is hard requirement), GPUI (0 releases in 8.5+ months), Xilem (zero production users).

No framework is committed as the final choice in M0. The egui prototype must demonstrate a 200k-row virtualized list with an a11y smoke test before a final Sol review.

## Evidence

- `gui-evaluation.md` lines 27-34 (framework landscape table) and line 49 (egui "100K records without any visible performance delay" benchmark): egui (immediate, AccessKit 0.35+, natural virtualization), Slint (declarative, built-in ListView, AccessKit default), iced (no stable AccessKit, no built-in virtualization, "frames dropped, sluggish at 100k"), GPUI (0 releases in 8.5 months), Xilem (zero production users).
- `gui-evaluation.md` lines 64-84: AccessKit is the de-facto shared a11y layer; egui has gaps but is usable; Slint/Tauri/GPUI/Xilem have working integration; iced's a11y issue #552 open since 2020.
- `gui-evaluation.md` lines 49-55: egui immediate mode virtualizes naturally; iced requires custom virtualization work.
- `soundcraft.md` line 56 (ADOPT: "Keep UI dependencies out of the library/domain and adapters") and line 70 (REJECT: "Incumbent-menu parity counts as OxiDJ acceptance"); egui was used in SoundCraft's `ui-egui` crate but this is a research observation, not a port.
- `gui-evaluation.md` lines 126-131: "Prototype in egui" is the research recommendation; "evaluate Slint" is the alternative; "defer Tauri unless team has web expertise."

## Options Considered

| Option | Status | Reason |
|--------|--------|--------|
| egui/eframe prototype first | **Prototype candidate 1** | Fastest iteration, natural virtualization, AccessKit mandatory |
| Slint evaluate | **Prototype candidate 2** | Declarative, built-in virtualization, design tool |
| Tauri 2 | Deferred | Web-tech team availability, IPC complexity |
| iced | Rejected | No stable a11y (6+ years), no built-in virtualization |
| GPUI | Deferred | 0 releases in 8.5 months |
| Xilem | Deferred | Zero production users |

## Consequences

- If egui prototype passes the 200k-row + a11y smoke test, it becomes the provisional M3 framework choice (final Sol review).
- If egui fails a11y or performance gates, Slint is the fallback.
- No final dependency is added to the M0 workspace; the prototype lives in a scratch crate.
- egui's documented a11y gaps (text-input caret/RTL/BiDi) become open risks to document.

## Open Questions / What Would Change This

- Does egui's AccessKit integration expose enough semantics for NVDA/VoiceOver/Orca on all three platforms? (Requires real screen-reader testing.)
- Does iced ship stable AccessKit before M3? (Currently blocked since 2020.)
- Would a web-tech team make Tauri the better choice? (Decision gate: team composition by M3.)
- Linux Wayland + NVIDIA WebKitGTK bugs affecting Tauri — does this impact egui/Slint similarly?

## Sol Review Packet

> **Review status: SUSPENDED.** The required independent Sol/high review has not been performed and is suspended under the Nous-only free-model routing policy (see M0-07 comment, 2026-10-15). This packet is prepared for when the review gate is restored; do not treat the recommendation as reviewed.

**DECISION:** Prototype in egui first for M3 library UI; evaluate Slint as alternative; defer Tauri (unless web-tech team), iced (a11y gap), GPUI (stalled), Xilem (no production users). Final framework commitment gated on 200k-row virtualized list + a11y smoke-test results.

**WHY NOW:** M3 requires a GUI. Delaying the prototype past early M3 risks rework if virtualization or a11y fails at scale. The research evaluation (gui-evaluation.md) provides the evidence base.

**CONSTRAINTS:** Must support 100k–500k track virtualization at 60fps. Must integrate AccessKit for cross-platform a11y. Must work on Linux (primary), macOS, Windows. No framework may become a hard dependency until prototype validates it. No M5 or realtime-audio GUI in M0.

**OPTIONS:** (1) egui/eframe — prototype candidate 1; (2) Slint — evaluate candidate 2; (3) Tauri 2 — defer unless web team; (4) iced — rejected (no stable a11y); (5) GPUI/Xilem — defer (stalled / no production users).

**EVIDENCE:** gui-evaluation.md: virtualization benchmarks (egui 100k smooth, iced 100k sluggish), AccessKit status table (egui mandatory 0.35+, Slint default, iced missing since 2020), maintenance-risk signals (notgull burnout, GPUI 8.5mo no release, Xilem zero production users). soundcraft.md: ADOPT layer separation, REJECT menu parity.

**CURRENT RECOMMENDATION:** Prototype egui first; evaluate Slint. Confidence: medium (prototype gate remaining). What changes it: egui a11y smoke test failure, 200k-row performance below 60fps, or iced shipping stable AccessKit before M3.

**OUTPUT CONTRACT:** A scratch crate `prototypes/oxidj-gui-probe` that builds on Linux/macOS/Windows, renders 200k synthetic tracks in a virtualized list, runs an AccessKit tree smoke test. Produce a short report: build/test commands, frame-rate measurement, a11y tree status. No dependency enters `Cargo.toml` at workspace root — this is a throwaway probe.

## Independent Review Outcome (2026-10-15)

**Reviewer:** `gpt-6.1-sol` via `openai-codex`, reasoning **low**, isolated delegate (no tools, no delegation, this decision only).

**Recommendation returned:** **egui/eframe, prototype first.** Compare it against Slint before committing, and commit now only to a **UI-independent, command-first application layer**. Confidence: **medium**.

**Reasons accepted:**

- egui is the strongest first candidate on Rust-native development, virtualized rendering and permissive licensing — but **virtualization alone does not establish 60fps with OxiDJ's actual workload.**
- **Accessibility is the gating risk.** Mandatory AccessKit integration does not prove usable screen-reader navigation, focus management, or accessible table semantics. This is the requirement most likely to overturn the choice.
- Slint is a serious challenger and may be the better-balanced choice if its accessibility and keyboard interaction prove better in practice. The evidence does not establish a winner across all requirements.
- **Qt: defer, not disqualify.** Qt's maturity does not transfer automatically to `cxx-qt`. On the evidence, `cxx-qt` is an actively maintained candidate but not a proven stable Rust integration; API churn warrants deferral, and the Qt Bridges uncertainty is an additional risk rather than a permanent veto.
- **Slint's GPL/commercial terms are a conditional constraint, not a veto.** They become decisive only if *both* GPL-compatible distribution and commercial licensing are unacceptable for OxiDJ.

**Counterargument recorded:** egui may turn the central library table into substantial custom interaction and accessibility work. If accessibility is non-negotiable, choosing egui primarily for rendering simplicity could optimise the wrong problem — Slint, or Qt with an accepted binding-maintenance cost, could be the better foundation.

**Consequence:** the prototype gate stands, but it is **not a scrolling benchmark**. Committing to egui requires the same interaction and accessibility checks that would justify Slint: screen-reader table navigation, keyboard multi-selection, drag-and-drop, *while* meeting the 500k-row performance target.

**What would change the decision:** a representative comparison showing Slint with better screen-reader table navigation, keyboard multi-selection and drag-and-drop while meeting the performance target and with acceptable licensing; or `cxx-qt` gaining credible API stability, or OxiDJ explicitly accepting its upgrade burden.

## Licensing resolution (2026-10-15) — the conditional constraint fired

`ADR-009` licences OxiDJ under **MIT**. The review above held that Slint's GPL-3.0/commercial terms become decisive only if *both* GPL-compatible distribution and commercial licensing are unacceptable. MIT makes GPL-compatible distribution unacceptable and no commercial licence is being purchased, so **the condition has fired**:

- **Slint is excluded at the free tier.** It remains reachable only by purchasing a commercial licence — an explicit commercial decision, not a technical one.
- **egui/eframe is therefore the framework to prototype**, which is what the review recommended anyway. The prototype gate is unchanged: screen-reader table navigation, keyboard multi-selection and drag-and-drop must pass *alongside* the 500k-row performance target.
- Qt/LGPL remains theoretically compatible with MIT, but Qt stays deferred on API-stability grounds.
