# OxiDJ

OxiDJ is a modern, open-source DJ library and eventually performance application written primarily in Rust.

The long-term goal is to become a credible Rekordbox alternative without locking the user into one vendor ecosystem.

## Priorities

1. trustworthy local DJ library management
2. strong OneLibrary / Rekordbox interoperability
3. excellent cross-platform desktop UX
4. DJ metadata including BPM, tempo, beat grids, cues, loops and key
5. analysis
6. realtime DJ functionality later

## Principles

- Rust-first, not Rust-at-all-costs
- local-first
- Linux-first development, cross-platform architecture
- no mandatory cloud account
- vendor formats behind adapters
- neutral internal domain model
- no silent metadata loss
- reproducible import/export and round-trip tests
- UI separated from business logic
- realtime audio isolated from ordinary application code
- mature libraries before unnecessary reimplementation

## MVP

The first major product proof is:

> Import a real DJ library, manage it safely in OxiDJ, export it again, and continue using compatible DJ software/hardware without unexpected metadata loss.

The MVP is library-first.

Full decks, realtime mixing, effects, stems, streaming, broad controller support and similar performance features are intentionally deferred.
