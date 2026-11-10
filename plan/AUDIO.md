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
| A1 | Output selection: `OutputConfig` (host, device, rate, buffer, channels), `list_outputs()`, `Engine::start_with`, `Engine::reopen` with deck state carried, automatic reconnect after device loss | done c879e38 |
| A2 | Stats: render time, DSP load, output latency, actual buffer; `examples/qualify` takes the config and prints them | done c879e38 |
| A3 | Declick: short fades on play, pause, seek, cue jump, loop wrap | done d569041 |
| A4 | Windowed-sinc resampler in-crate replacing linear interpolation (exact copy kept at ratio 1) | done d569041 |
| A5 | Master limiter replacing the bare clamp (clamp stays as last safety) | done (third commit) |
| A6 | Cargo features `asio`, `jack`, `realtime`, `realtime-dbus` forwarding to cpal; host selection by name; build notes in `docs/audio-output.md` | done (third commit) |
| A7 | ADR-008 addendum with decisions and measurements; this file updated | done (third commit) |
| A8 | Independent review of the branch (Anthropic model, Kanban card) | todo |
| A9 | WASAPI exclusive mode: needs cpal support or a patch; decide later | parked |
| A10 | Streaming or i16 track storage to cut RAM | parked |
| A11 | UI wiring: Settings for output device, buffer, limiter; status line with latency and load | parked (UI session) |
| A12 | Spread key-lock FFT work across callbacks. Measured: cheaper preset cuts the worst block from 1.2 ms to 0.5 ms, but `key_lock_tempo_change_just_before_a_beat_stays_on_the_click` fails (6.0 ms against a 5 ms bound). Needs either `splitComputation` for the default preset (wrapper 0.1.3 does not expose it) or a re-derived position model plus a listening test | parked |
| A13 | Build the installed app with `jack` (Linux) and prefer the JACK host when present; needs a change in the UI crate's Cargo features and `scripts/install-local.sh` | parked (UI session) |
| A14 | Limiter look-ahead (about 1 ms) so the first over of a burst is anticipated | parked, only if listening shows a need |
| A15 | Real-hardware qualification: USB card unplug and replug, 4-channel cue through `OutputConfig`, 10-minute `qualify` on JACK, Windows ASIO build and run | needs owner hardware |
| A16 | Split cue for 2-channel cards (master left, cue right) and free channel-pair routing | parked, YAGNI until asked |

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
- A3 + A4 done (d569041). Declick is on for real outputs only; `Engine::manual`
  stays sample-exact so the existing 45 tests did not change.
- A5 + A6 + A7 done. New measurements (`examples/bench`, JACK host) are in the
  ADR-008 addendum table. Key results: engine without key lock costs 0.6 % of a
  block; key lock bursts 1.2 ms; JACK host at 256 frames gives 8.0 ms latency,
  worst load 21 %, 0 xruns. cpal's `realtime` promotion applies to ALSA
  hardware devices only, not to the PipeWire bridge (checked with `ps -L`:
  `cpal_alsa_out` stayed `TS`), so on PipeWire the JACK host is the realtime path.
- A key-lock toggle on a playing deck is declicked too (same position, different signal path).
- D7 Limiter has a switch (`Command::Limiter`) and is on by default.
- D8 `.github/workflows/ci.yml` installs `libjack-jackd2-dev` and `libdbus-1-dev`
  because the gate builds with `--all-features`.

## How to continue (next session)

1. `cd /home/senci/dev/oxidj/.worktrees/audio-engine && git status && git log --oneline master..`
2. Read the work rows above. Everything not `done` is `parked` with its reason.
3. If the review card (A8) returned findings, fix them on this branch, one
   commit per finding, gate each.
4. Merge is the owner's call: `git merge audio-engine` on `master` when the UI
   session's checkout is clean. Expected conflicts: none in the UI crate (not
   touched); `Cargo.lock` may need `cargo update -w` or a re-resolve.

