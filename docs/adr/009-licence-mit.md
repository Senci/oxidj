---
id: 009
title: Licence OxiDJ under MIT
status: Accepted
date: 2026-10-15
owners: [oxidj-2]
decided_by: user
sources: [docs/adr/001-gui-framework.md, docs/adr/008-audio-engine.md]
sol_review: not-applicable
---

# ADR-009: Project Licence — MIT

> **Accepted.** This decision was made by the project owner (Senad Licina), not by the lead. It is recorded here because it constrains several other decisions.

## Context

OxiDJ had no licence file. That was not cosmetic: Slint (GPL-3.0 or commercial), Rubber Band (GPL-2.0+ or commercial) and Qt (GPL/LGPL/commercial) are all licence-conditional, so the project licence decided which of the researched options were actually available. The choice had to be made before either the UI framework or a GPL time-stretch engine could be committed to.

## Decision

OxiDJ is licensed under the **MIT License**. See `LICENSE` at the repository root. Copyright holder: Senad Ličina.

## Consequences

**Now excluded (free tiers) and must not be introduced without a commercial licence:**

| Component | Licence | Status under MIT |
|---|---|---|
| **Slint** | GPL-3.0 or commercial | Excluded at the free tier. The UI framework therefore falls to **egui/eframe** (MIT/Apache), with Slint reachable only by buying a commercial licence. |
| **Rubber Band** | GPL-2.0+ or commercial | Excluded. Independently already rejected — its Rust bindings have 208 lifetime downloads, so the Rust path is unproven regardless of licence. |
| **Qt / cxx-qt** | GPL/LGPL/commercial | LGPL linking is generally compatible with MIT, but Qt was already deferred on API-stability grounds. Revisit only with a licensing review. |

**Now required:**

- **A permissive-only dependency policy, enforced.** `cargo deny check licenses` must run with an explicit allowlist (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-DFS-2016, Zlib) and must **deny** GPL/LGPL/AGPL/SSPL by default. This is a build gate, not a convention — an accidental copyleft dependency silently changes the licence of the distributed binary. Wired into `M0-08`'s quality gate.
- Every new dependency needs its licence checked at the point it is added, not at release.
- `license = "MIT"` belongs in the workspace `Cargo.toml` (M0-08).

**This resolves the conditional constraint in ADR-001.** The GUI review held that Slint's GPL/commercial terms become decisive only if *both* GPL-compatible distribution and commercial licensing are unacceptable. MIT makes GPL-compatible distribution unacceptable, and no commercial licence is being purchased — so the condition has fired, and **egui/eframe is the framework to prototype**.

**ADR-008 is unaffected in substance but settled on the licensing question:** Signalsmith Stretch (MIT) is compatible; Rubber Band is now excluded on licence *and* binding maturity.

## Open question (trivial to change now, painful later)

Rust convention is often dual `MIT OR Apache-2.0`, where Apache-2.0 adds an explicit patent grant that some corporate contributors require. Single MIT is a valid and common choice. Adopting the dual licence is a one-line change **now** and requires contributor agreement once outside contributions exist. Left as a noted option rather than acted on, since the owner specified MIT.
