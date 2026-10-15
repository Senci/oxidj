---
id: 008
title: Audio engine foundation — CPAL I/O with a replaceable time-stretch engine
status: Provisional
date: 2026-10-15
owners: [oxidj-2]
sources: [docs/research/audio-platform-evaluation.md]
sol_review: required
review_status: completed 2026-10-15 (gpt-6.1-sol, reasoning low) — Option 1 provisional, confidence medium
review_confidence: medium
provisional_reason: The review supports CPAL + Signalsmith as a candidate, not a production qualification. DJ-grade latency, device recovery and stretch quality on real DJ material are all unmeasured.
---

# ADR-008: Audio Engine Foundation

> **Provisional.** No realtime audio is implemented in M0. This ADR fixes the *boundary* and the *candidate* foundation so later work does not have to rework the library and analysis crates.

## Context

OxiDJ needs two decks with sample-accurate scheduling, tempo-matching time-stretch and pitch-shift, low-latency monitoring, and cross-platform device handling (PipeWire/JACK on Linux, WASAPI with opt-in ASIO on Windows, CoreAudio on macOS). The engine arrives at a later milestone, but the dependency and data-boundary choices constrain everything built before it.

OxiDJ has **no licence file yet**. That is not incidental: it decides whether GPL-licensed components are usable at all.

## Decision

CPAL for device I/O, with the time-stretch engine behind an engine-independent trait, and Signalsmith Stretch as the first implementation.

## Evidence

| Crate | Version | Downloads | Updated | Note |
|---|---|---|---|---|
| `cpal` | 0.18.2 | **24,342,358** | 2026-08-16 | De-facto foundation of the Rust audio ecosystem; backends ALSA/PulseAudio/PipeWire/JACK/WASAPI/ASIO/CoreAudio/AAudio/WebAudio; `realtime` feature for RT thread scheduling; I/O only, no mixing or DSP |
| `rubato` | 5.0.1 | **13,183,853** | 2026-10-01 | Resampling; healthy and active |
| `cubeb` | 0.38.0 | 525,515 | 2026-08-18 | Mozilla's engine; real, but ~46x fewer downloads than cpal and no demonstrated advantage in the evidence |
| `fundsp` | 0.23.0 | 228,606 | 2026-01-07 | DSP graph; slower release cadence |
| `signalsmith-stretch` | 0.1.3 | 117,132 | 2025-09-18 | MIT wrapper for the Signalsmith algorithm; early version but genuinely used |
| `rubberband` | 0.1.5 | **208** | 2026-09-18 | GPL-2.0+/commercial **and** the Rust bindings are effectively unproven |

The `rubberband` finding is decisive on its own: 208 lifetime downloads means the Rust path is untested in practice, quite apart from the licence. Avoid it for both reasons, not just one.

## Independent Review Outcome (2026-10-15)

**Reviewer:** `gpt-6.1-sol` via `openai-codex`, reasoning **low**, isolated delegate (no tools, no delegation, this decision only).

**Recommendation returned:** Option 1, **provisionally** — CPAL for device I/O, Signalsmith Stretch behind an engine-independent trait. Select other DSP dependencies only when needed; **resampling is not a substitute for independent tempo/pitch control**.

**Confidence: medium.** The reviewer's central caution: *"CPAL is a reasonable Rust-first default, **not proof of DJ-grade performance**."* Backend availability, multichannel routing, callback jitter, device recovery and actual monitoring latency all require validation. Neither cubeb nor miniaudio showed a demonstrated advantage in the supplied evidence.

**Licensing reasoning accepted:** Signalsmith's MIT licence preserves permissive options. Distributing a linked Rubber Band implementation under GPL terms would generally require a GPL-compatible combined application; staying permissive needs a commercial licence or a different implementation. GPL acceptance would remove Rubber Band's licensing obstacle — **but would not by itself establish technical superiority.**

**Trait design requirement accepted:** the stretch trait must expose independent tempo and pitch controls, preparation limits, latency, input/output frame accounting, and reset and drain semantics. Allocation and configuration happen outside rendering — **the trait alone cannot make an implementation realtime-safe.**

**Counterargument recorded:** Signalsmith's reported quality does not establish acceptable transient preservation, scratching behaviour, or bounded processing cost on DJ material. CPAL also leaves substantial device-management work to OxiDJ. A commercially licensed proven engine — or cubeb/miniaudio, if device tests favour them — could reduce shipping risk despite the integration cost.

## M0 boundary requirements (the actionable output of this review)

This is what M0 must actually define, per the review:

1. **Separate library/analysis data from engine/device types.** Stable track identities, source-frame positions, versioned beat-grid and analysis units, and an explicit conversion to a shared engine-frame clock.
2. **Define bounded, timestamped commands** between application and engine.
3. **Preallocated PCM transfer**, channel routing, and defined underrun behaviour.
4. **Off-callback decoding and device recovery** — never in the audio thread.
5. **Sample-accurate deck scheduling on the shared clock**, accounting for stretch latency.
6. **Keep both I/O and stretch replaceable.** An Elastique adapter is architecturally sensible, but commercial availability and distribution rights must be established separately.
7. **Document Bluetooth output as unsupported.**

## Open risks

- No measured latency/xrun target exists yet; "low latency" is currently a wish, not a number.
- Signalsmith Stretch quality is unmeasured on DJ material, especially transients and scratching.
- CPAL device-management burden on OxiDJ is unbounded until device tests run.
- Real-time thread priority on Linux may need `rtkit`/realtime D-Bus configuration and user setup.
- ASIO on Windows requires the proprietary SDK and an LLVM/Clang toolchain — opt-in feature, documented build steps.

## What would change the decision

**What would change the decision:** comparative tests showing CPAL missing an explicitly agreed latency/xrun target, or required routing/recovery behaviour, while cubeb or miniaudio meets it; Signalsmith failing blinded DJ-material quality tests or worst-case processing budgets; or affordable commercial licensing plus demonstrably better results from Rubber Band or Elastique.

## Licensing resolution (2026-10-15)

`ADR-009` licences OxiDJ under **MIT**. Rubber Band is therefore excluded on **two independent grounds** — its GPL-2.0+/commercial licence is incompatible with MIT distribution, and its Rust bindings (`rubberband` 0.1.5) have **208 lifetime downloads**, meaning the Rust path is unproven regardless. Signalsmith Stretch (MIT) is compatible and remains the first implementation behind the replaceable trait. A commercial engine such as Elastique would require an explicit commercial decision and is not contemplated now.
