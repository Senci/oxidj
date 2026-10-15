# ADR Template

> M0-07 decision packet format. Each ADR is also a scoped Sol/high review packet.
> Keep the packet body to ~300-800 words. Do not bundle multiple decisions per ADR.

## Frontmatter

```
---
id: <sequential number, zero-padded>
title: <short, imperative>
status: Accepted | Provisional | Proposed | Superseded by ADR-XXX | Deprecated
date: YYYY-MM-DD
owners: [oxidj-2]
sources: [see inline citations]
sol_review: required | not-required
provisional_reason: <short reason if not yet accepted, e.g. "Sol review required but not yet completed; suspended under Nous-only free-model routing" or "M0-07 integration gate not yet independently signed off">
---
```

## Body (required sections)

1. **Context** — what drives this decision, in 2-3 sentences.
2. **Decision** — the accepted or proposed decision, stated as a directive.
3. **Evidence** — bullet list of cited evidence (research doc rows, source URLs, measurements).
4. **Options Considered** — what was rejected and why, one line each.
5. **Consequences** — positive and negative, bounded to what changes if adopted.
6. **Open Questions / What Would Change This** — explicit unknowns and the empirical bar.
7. **Sol Review Packet** — scoped packet in the format: DECISION, WHY NOW, CONSTRAINTS, OPTIONS, EVIDENCE, CURRENT RECOMMENDATION, OUTPUT CONTRACT.

## Status semantics

- **Accepted** — evidence-backed, independently reviewed, committed for the stated horizon.
- **Provisional** — evidence-backed recommendation that is **not yet accepted**. Used when an independent review gate is required but not yet completed (e.g. Sol/high review suspended under current model routing) or when the M0-07 integration gate has not been independently signed off. A Provisional decision must NOT drive implementation until it transitions to Accepted.
- **Proposed** — under active consideration; lacks a stable evidence base.
- **Superseded / Deprecated** — replaced or withdrawn.

## Rules

- One consequential decision per ADR.
- No speculative implementation (M5, realtime audio, GUI wiring) in M0.
- Every decision must rest on cited evidence, not a model claim.
- Speculative choices are listed as Open Questions, not assumed.
- Per the M0-07 integration gate and the Nous-only free-model routing policy, all M0 directions are recorded as **Provisional** until independent review is restored and completed. Do not treat any M0 ADR as Accepted architecture until its `provisional_reason` is resolved.
