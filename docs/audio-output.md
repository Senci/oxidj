# Audio output: devices, latency, backends

How to pick an output, get low latency and build the optional backends.
Engine code: `crates/oxidj-audio`. Plan and measurements: `plan/AUDIO.md`.
Decisions: ADR-008 (live-readiness addendum).

## Choosing the output

`Engine::start()` reads these environment keys; a settings screen passes the
same values as `OutputConfig` to `Engine::start_with` or `Engine::reopen`
(decks keep playing state across a switch).

| Key | Meaning | Default |
|---|---|---|
| `OXIDJ_AUDIO_HOST` | `ALSA`, `JACK`, `ASIO`, `WASAPI`, `CoreAudio` (case-insensitive; must be compiled in) | platform default |
| `OXIDJ_AUDIO_DEVICE` | device id (preferred) or exact name | host default device |
| `OXIDJ_AUDIO_RATE` | sample rate in Hz | device default |
| `OXIDJ_AUDIO_BUFFER` | callback size in frames | host default (often 512 to 1024) |
| `OXIDJ_AUDIO_CHANNELS` | 2 = master only, 4 = master on 1/2 and headphone cue on 3/4 | device default |
| `OXIDJ_AUDIO_MASTER` | first output of the main pair, counted from 1 (3 = outputs 3/4) | 1 |
| `OXIDJ_AUDIO_CUE` | first output of the headphone cue pair, counted from 1 | 3 |
| `OXIDJ_AUDIO_SPLIT_CUE` | `1` = split cue for a 2-channel card: main in mono on the left output, cue in mono on the right | off |

A named device without rate or channels asks for 4 channels at 48 kHz.
Main outputs the device does not have, or main and cue pairs that overlap,
are an error too; a missing cue pair only leaves cue silent.
A rate or channel count the device does not offer is an error: the engine
runs silently, shows the reason and retries every 2 seconds.

List devices, ids, channel counts and buffer ranges:

    cargo run --release -p oxidj-audio --example qualify -- --list

## Latency guide

| Buffer (48 kHz) | Block time | Use |
|---|---|---|
| 512 | 10.7 ms | safe on any machine |
| 256 | 5.3 ms | recommended for playing live (set it; the engine does not pick it by itself) |
| 128 | 2.7 ms | fine without key lock; with key lock on both decks the worst steady block uses about 20 % of its time, a block with a jump about 60 % |
| 64 | 1.3 ms | varispeed only |

Key lock (Signalsmith) does its transform in bursts of about 0.6 ms per deck
every 30 ms; the engine keeps the two decks' bursts 15 ms apart. Everything
else in the engine costs under 1 % of a block.

## Linux

- Default: ALSA through PipeWire. Works everywhere, but the audio thread is
  not realtime and the bridge adds about 10 ms. Measured 13 ms at 128 frames,
  21 ms at 256.
- Recommended for playing: build with `--features jack` and run with
  `OXIDJ_AUDIO_HOST=JACK`. PipeWire's JACK layer runs the callback on its
  realtime thread. The buffer is PipeWire's quantum, set it with
  `PIPEWIRE_QUANTUM=256/48000` (measured: 8.0 ms latency, 0 xruns, worst load
  21 % with two key-locked decks; 4.0 ms at `128/48000`).
- A raw `hw:` device also works (`OXIDJ_AUDIO_DEVICE=hw:CARD=... OXIDJ_AUDIO_CHANNELS=2` for a
  stereo card; a named device alone asks for 4 channels), and with
  `--features realtime-dbus` cpal asks RealtimeKit for a realtime thread. cpal
  only does this for hardware devices, not for the PipeWire bridge.

## Windows

- Default: WASAPI shared mode. cpal 0.18.2 has no exclusive mode (plan A9).
- ASIO: build with `--features asio`. Needs LLVM/Clang (`LIBCLANG_PATH`) and
  the Steinberg ASIO SDK (cpal downloads it, or set `CPAL_ASIO_DIR`). Run with
  `OXIDJ_AUDIO_HOST=ASIO` and set the buffer in the driver panel or with
  `OXIDJ_AUDIO_BUFFER`. NOT RUN: no Windows build or ASIO hardware was
  available when this was written; the feature only forwards to cpal.

## macOS

CoreAudio is the default host; `OXIDJ_AUDIO_BUFFER` sets the buffer. NOT RUN.

## Measuring

    cargo run --release -p oxidj-audio --example bench          # DSP cost only
    cargo run --release -p oxidj-audio --example qualify -- 600 # real device

`qualify` plays two key-locked decks through the mixer with the master at 0
(silent) and prints block size, worst gap, xruns, worst and mean load and the
latency the device reports. Add `--varispeed` to leave key lock out.
`Engine::stats()` gives the same numbers to a UI.
