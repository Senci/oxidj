# Audio engine: live-performance readiness

Workstream started 2026-11-10 by the technical lead (owner asleep, autonomous).
This file is the handoff: read it first, then `git log master..audio-engine`.

- Branch `audio-engine`, worktree `/home/senci/dev/oxidj/.worktrees/audio-engine`.
  Not merged to `master` (the UI session owns the main checkout). Owner merges.
- Kanban board `oxidj-audio` (`hermes kanban --board oxidj-audio list`). Cards
  carry the same ids as the table below. Cards have no assignee on purpose, so
  the dispatcher never starts a second writer in this worktree.
- Gate per commit: `cargo xtask ci` in the worktree.
- Any model can continue: each open row below names the files, the behaviour
  and the test that proves it. Do one row, run the gate, commit, tick the row.

## Owner decisions (2026-11-10)

| Topic | Decision |
|---|---|
| Landing | Branch `audio-engine`, pushed, not merged |
| UI crate | Compile fixes only, when an engine signature changes |
| Headphone cue | In scope (already present in the engine: outputs 3/4) |
| Backends | ASIO, JACK and WASAPI exclusive behind cargo features |
| Dependencies | None new; resampler written in-crate |
| Review | One Anthropic review of the whole branch at the end |
| Delegation | Anthropic only. `delegate_task` is mapped to OpenAI models, so it is not used; the final review is a Kanban card pinned to an Anthropic model |
| Targets | Lead defines them (below) |
| Models | Match the model to the task: a routine-coding model (Sonnet class) for the mechanical rows below, a stronger model only for the final review (A8) and for anything that changes the realtime contract. No model or quota lookups beyond what a row needs |

## Targets (lead-defined)

| Target | Value | How it is measured |
|---|---|---|
| Requested buffer | 256 frames default; 64 to 2048 selectable | `OutputConfig::buffer_frames`, actual size in `StatsSnapshot` |
| Output latency | 10 ms or less on ASIO/JACK at 256 frames/48 kHz; 20 ms or less on shared-mode hosts | `StatsSnapshot::latency_ns` (device timestamp) |
| Dropouts | 0 xruns in 10 min, two decks, key lock + mixer, idle machine | `examples/qualify` |
| DSP load | Worst render time 30 % or less of the block period at 128 frames/48 kHz | `StatsSnapshot::max_render_ns` |
| Control delay | A command is applied in the next callback | by design (command ring drained per callback) |
| Device loss | Decks keep running; the device is retried every second and taken back | `Engine::poll` |
| Sample rates | 44.1, 48, 88.2, 96 kHz device rates | `OutputConfig::sample_rate` |
| Clicks | No full-scale step on play, pause, seek, cue or loop wrap | declick tests |

Real ASIO hardware, Windows and macOS are NOT RUN on this machine (Linux).

## Audit findings (2026-11-10, commit a13ada4)

Good, keep: lock-free command ring, atomics snapshots, no allocation in the
callback (`tests/rt_alloc.rs`), key lock with latency-compensated position,
mixer with ramped gains and denormal flush, cue bus on outputs 3/4, silent
fallback with deck state carried over.

| # | Finding | Impact live | Card |
|---|---|---|---|
| F1 | Only the default device, or `OXIDJ_AUDIO_DEVICE` which demands exactly 4 channels at 48 kHz. No host, rate or buffer choice, no device list | Cannot pick the sound card or a low-latency buffer | A1 |
| F2 | `BufferSize::Default` always (measured 512 to 1024 frames on PipeWire) | 10 to 40 ms latency | A1 |
| F3 | Device loss falls back to silence for ever; nothing retries the device | A pulled USB cable ends the set until restart | A1 |
| F4 | No measurement of render time or output latency | Cannot qualify a buffer size | A2 |
| F5 | Play, pause, seek, cue and loop wrap switch the signal on a single sample | Audible clicks | A3 |
| F6 | Linear interpolation for 44.1 to 48 kHz and varispeed | Treble loss (about 3 dB near 20 kHz) and imaging | A4 |
| F7 | Master is hard-clamped at full scale; no limiter | Harsh distortion when two hot tracks sum | A5 |
| F8 | No ASIO/JACK host can be compiled in; no RT thread priority | No low-latency path on Windows; xruns under load | A6 |
| F9 | cpal 0.18.2 has no WASAPI exclusive mode | Cannot be built without patching cpal | A9 (parked) |
| F10 | Tracks are held fully decoded in RAM as f32 (about 20 MB per minute) | Fine for 2 decks; revisit for 4 decks | A10 (parked) |

## Work rows

Status: `todo`, `doing`, `done <sha>`, `parked`.

| Id | Row | Status |
|---|---|---|
| A1 | Output selection: `OutputConfig` (host, device, rate, buffer, channels), `list_outputs()`, `Engine::start_with`, `Engine::reopen` with deck state carried, automatic reconnect after device loss | done (first commit on branch) |
| A2 | Stats: render time, DSP load, output latency, actual buffer; `examples/qualify` takes the config and prints them | done (first commit on branch) |
| A3 | Declick: short fades on play, pause, seek, cue jump, loop wrap | todo |
| A4 | Windowed-sinc resampler in-crate replacing linear interpolation (exact copy kept at ratio 1) | todo |
| A5 | Master limiter replacing the bare clamp (clamp stays as last safety) | todo |
| A6 | Cargo features `asio`, `jack`, `realtime` forwarding to cpal; host selection by name; build notes | todo |
| A7 | ADR-008 addendum with decisions and measurements; this file updated | todo |
| A8 | Independent review of the branch (Anthropic model, Kanban card) | todo |
| A9 | WASAPI exclusive mode: needs cpal support or a patch; decide later | parked |
| A10 | Streaming or i16 track storage to cut RAM | parked |
| A11 | UI wiring: Settings for output device, buffer, limiter; status line with latency and load | parked (UI session) |
| A12 | Spread key-lock FFT work across callbacks (needs `splitComputation` exposed by the wrapper, or the cheaper preset after a listening test) | parked |

## Decisions log

- D1 Output choice reaches the engine two ways: `Engine::start_with(OutputConfig)`
  for a settings screen, and environment keys (`OXIDJ_AUDIO_HOST`, `_DEVICE`,
  `_RATE`, `_BUFFER`, `_CHANNELS`) read by `Engine::start()`, so the current UI
  gets every option without a UI change.
- D2 A device is addressed by its stable cpal id (`OutputInfo::id`) or by name.
  ALSA repeats names across endpoints, so name-only selection was often refused.
- D3 A requested rate or channel count the device lacks is an error (silent
  fallback plus retry), never a quiet downgrade: a missing cue pair must be seen.
- D4 Reconnect runs from `Engine::poll` every 2 s on the UI thread while a wanted
  device is missing. It enumerates devices, which can take some milliseconds;
  acceptable because audio is already down. An engine that never had a device
  (`Engine::silent`, tests) never retries.
- D5 `Engine::reopen` closes the old stream before opening the new one, because
  ASIO and ALSA `hw:` devices refuse a second stream.
- D6 Key lock keeps the Signalsmith default preset. The `cheaper` preset (the
  only one in wrapper 0.1.3 that splits the FFT work across callbacks) showed
  no clear gain in three 12 s runs at 128 frames; worst-case numbers on this
  desktop are dominated by scheduling, not DSP. Revisit after RT priority (A6).

## Progress log

- A1 + A2 done. `cargo test -p oxidj-audio`: 39 unit + 5 cue + 1 rt_alloc pass.
  Measured here (PipeWire through cpal ALSA, 48 kHz, two key-locked decks +
  mixer, no RT priority, machine also running other builds), 12 to 15 s runs:

  | Requested buffer | Actual block | Reported latency | Worst render | Mean of worst-load runs |
  |---|---|---|---|---|
  | default | 512 | 37.3 ms | 1.27 ms | 12 % worst load |
  | 256 | 256 | 21.3 ms | 8.1 ms (one outlier) | |
  | 128 | 128 | 13.3 ms | 1.2 to 3.6 ms | 44 to 136 % worst load |
  | 128, varispeed (no key lock) | 128 | 13.3 ms | 0.27 to 0.47 ms | 10 to 17 % worst load |

  Reading: buffer selection works and halves latency per step. The PipeWire
  ALSA bridge adds about 10 ms on top of the block, so the 10 ms target needs
  JACK/ASIO or a `hw:` device. Key lock costs about 1 to 2 ms in its worst
  callback (both decks' FFT hops coinciding), so 128 frames is marginal with
  key lock and 256 is the safe default. 0 xruns in all runs.
- F11 (new finding): key-lock render time is bursty; see D6. Card A12 parked.
