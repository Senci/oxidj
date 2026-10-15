# M0-06: Audio / Platform Boundaries Evaluation

**Status:** Research complete — review ready
**Owner:** oxidj-2
**Date:** 2026-10-15

---

## Executive Summary

OxiDJ needs to define the boundary between:
- **Library/analysis code** (offline, batch, no realtime constraints)
- **Realtime audio engine** (decks, mixing, effects, low-latency I/O)

This evaluation covers the **CPAL** ecosystem for cross-platform audio I/O and the **decode/DSP** library landscape. **No realtime implementation in M0** — this is boundary definition only.

**No decision without scoped Sol review.**

---

## CPAL: Cross-Platform Audio I/O

### What CPAL Provides

| Capability | Details |
|------------|---------|
| **Device enumeration** | Hosts, devices, default input/output |
| **Stream configuration** | Sample rate, channel count, sample format (compile-time or runtime) |
| **Callback-based I/O** | Real-time audio callback (separate thread) |
| **Stream control** | Play, pause, buffer size, clock query |

### Platform Backend Matrix (CPAL 0.18+)

| Platform | Default Backend | Optional Backends | Feature Flags | System Deps |
|----------|-----------------|-------------------|---------------|-------------|
| **Linux** | ALSA | JACK, PipeWire, PulseAudio | `jack`, `pipewire`, `pulseaudio` | `libasound2-dev` (always), `libjack-dev`, `libpipewire-0.3-dev`, `libpulse-dev` |
| **Windows** | WASAPI | ASIO, JACK | `asio`, `jack` | ASIO SDK, LLVM/Clang (ASIO) |
| **macOS** | CoreAudio | JACK | `jack` | None (system frameworks) |
| **iOS/tvOS/visionOS** | CoreAudio | — | — | None |
| **Android** | AAudio | — | — | NDK API 26+ |
| **WebAssembly** | Web Audio API | AudioWorklet | `wasm-bindgen`, `audioworklet` | — |
| **BSD** | ALSA | JACK, PipeWire, PulseAudio | Same as Linux | Same as Linux |

**Sources:** [CPAL README](https://github.com/RustAudio/cpal/blob/master/README.md), [DeepWiki Platform Backends](https://deepwiki.com/RustAudio/cpal/3-platform-backends), [DeepWiki Building](https://deepwiki.com/RustAudio/cpal/4-building-and-configuration).

### Backend Details

#### Linux
- **ALSA** (default): Kernel-level, lowest latency, exclusive access possible
- **PipeWire** (recommended for modern distros): Unified graph, supports JACK/PulseAudio apps, professional features
- **JACK**: Pro audio standard, inter-app routing, requires JACK server
- **PulseAudio**: Legacy, higher latency, being replaced by PipeWire

**OxiDJ recommendation:** Enable `pipewire` + `jack` features; ALSA always included. PipeWire is the future on Linux.

#### Windows
- **WASAPI** (default): Shared/exclusive modes, Windows 10+, no extra deps
- **ASIO** (optional): Pro audio, bypasses Windows stack, lower latency, requires ASIO SDK + LLVM/Clang for bindgen
- **JACK**: Possible but rare on Windows

**OxiDJ recommendation:** WASAPI default; ASIO as opt-in feature for pro users with ASIO interfaces.

#### macOS
- **CoreAudio** (default): Native, low latency, aggregate devices, loopback recording (macOS 14.6+)
- **JACK**: Available but uncommon

**OxiDJ recommendation:** CoreAudio only; JACK feature for niche cases.

### CPAL Architecture

```
Application
    │
    ▼
┌─────────────────────────────────────┐
│         CPAL Traits                 │
│  HostTrait / DeviceTrait /          │
│  StreamTrait                        │
└─────────────────────────────────────┘
    │           │           │
    ▼           ▼           ▼
┌────────┐ ┌────────┐ ┌────────┐
│ ALSA   │ │ WASAPI │ │CoreAudio│
│ JACK   │ │ ASIO   │ │ JACK   │
│PipeWire│ │        │ │        │
└────────┘ └────────┘ └────────┘
```

**Dispatch:** `impl_platform_host!` macro generates platform-specific enums + dynamic dispatch. `available_hosts()` detects at runtime.

### Realtime Considerations

| Concern | CPAL Status | OxiDJ Mitigation |
|---------|-------------|------------------|
| **Callback thread** | Real-time priority (platform-dependent) | `realtime-dbus` feature on Linux (requires `libdbus-1-dev`) |
| **Buffer sizes** | Platform/device dependent (typically 128–1024 frames) | Query `StreamConfig::buffer_size`; adapt DSP block size |
| **Sample formats** | f32, i16, u16, i24, i32 (device-dependent) | Convert in callback; use f32 internally |
| **Clock drift** | `Stream::clock()` provides frame position | Sync decks to master clock |

---

## Decode / DSP Boundaries

### OxiDJ Architecture Principle
> **Realtime audio isolated from ordinary application code.**

```
┌────────────────────────────────────────────────────────────────┐
│                    OxiDJ Process                               │
├──────────────────────────┬─────────────────────────────────────┤
│    Library / Analysis    │        Realtime Engine (Future)     │
│    (No realtime reqs)    │        (Separate crate, RT-safe)    │
├──────────────────────────┼─────────────────────────────────────┤
│ • File scanning          │ • CPAL stream callback              │
│ • Metadata extraction    │ • Deck playback (resample, sync)    │
│ • BPM/beatgrid analysis  │ • Mixer (EQ, filter, gain)          │
│ • Waveform generation    │ • Effects chain (RT-safe)           │
│ • Cue/loop management    │ • Output to CPAL device             │
│ • Import/export          │ • Zero-allocation hot path          │
└──────────────────────────┴─────────────────────────────────────┘
```

### Decode Libraries (Offline / Analysis)

| Library | Formats | Approach | Status | Use Case |
|---------|---------|----------|--------|----------|
| **Symphonia** | MP3, FLAC, AAC, ALAC, Vorbis, Opus, WAV, MP4, MKV, OGG, AIFF, CAF | Pure Rust demux + decode; feature-gated codecs | ✅ Active (0.6.0 May 2026) | **Primary decode** for analysis, waveform, metadata |
| **Rodio** | MP3, FLAC, WAV, Vorbis, MP4 (via Symphonia) | High-level playback; composable sources | ✅ Active | Simple playback; not for analysis |
| **Hound** | WAV only | Pure Rust WAV encode/decode | ✅ Stable | WAV-only tools |
| **Claxon** | FLAC only | Pure Rust FLAC decode | ✅ Stable | FLAC-only |
| **Minimp3** | MP3 only | Minimal MP3 decode | ✅ Stable | MP3-only |
| **Lewton** | Vorbis only | Pure Rust Vorbis decode | ✅ Stable | Vorbis-only |

**Symphonia is the clear winner** for OxiDJ:
- Single crate (`symphonia` umbrella) with feature flags
- Automatic format/codec probing
- Metadata/tag reading (`symphonia-metadata` crate)
- Gapless playback support (FLAC, ALAC, MP3, Vorbis, PCM, ADPCM)
- Pure Rust, no FFI to FFmpeg

**Sources:** [Symphonia GitHub](https://github.com/jiztastamablastamarang/symphonia), [DeepWiki Rodio](https://deepwiki.com/RustAudio/rodio), [openapps.pro Symphonia](https://openapps.pro/packages/symphonia), [lib.rs Symphonia](https://lib.rs/crates/symphonia).

### Symphonia Feature Flags (Opt-In)

```toml
# Cargo.toml
symphonia = { version = "0.6", features = [
    "ogg", "wav", "flac", "mp3", "vorbis", "opus",
    "isomp4", "mkv", "aiff", "caf",  # containers
    "aac", "alac", "adpcm",          # extra codecs
    "metadata",                      # tag reading
] }
```

### DSP Libraries (Realtime-Safe Candidates)

| Library | Category | RT-Safe? | Status |
|---------|----------|----------|--------|
| **dasp** | Signal processing primitives | ✅ Designed for RT | Active |
| **fundsp** | Audio DSP graph/language | ✅ RT-safe | Active |
| **valib** | VA filters, analog modeling | ✅ RT-safe | Active |
| **rubato** | Sample rate conversion | ✅ RT-safe | Active |
| **ears** | Simple playback | ❌ Not RT-focused | Maintenance? |
| **cpal** | I/O only | N/A | Active |

**For M0 boundary definition:** Document that realtime DSP will use `dasp`/`fundsp`/`rubato` — no implementation yet.

---

## Platform-Specific Audio Considerations

### Linux
- **PipeWire** is the convergence target (replaces PulseAudio/JACK for most users)
- **Realtime scheduling:** `realtime-dbus` feature + `libdbus-1-dev`; or `memlock` limit + `SCHED_FIFO` via `rtkit`
- **Device aggregation:** PipeWire supports aggregate/virtual devices natively
- **Bluetooth:** A2DP via PipeWire/BlueZ; high latency (~100–200ms) — not for DJ use

### macOS
- **CoreAudio** is excellent; aggregate devices in Audio MIDI Setup
- **Loopback recording:** macOS 14.6+ (CPAL requirement)
- **Pro Audio:** Low I/O buffer sizes (64–256 frames) achievable
- **Bluetooth:** AAC/aptX; latency ~150ms — not for DJ use

### Windows
- **WASAPI Shared Mode:** Default, ~10ms latency, mixed with system sounds
- **WASAPI Exclusive Mode:** Lower latency (~3–5ms), takes device control
- **ASIO:** Pro standard, ~1–3ms, bypasses Windows audio stack entirely
- **Device aggregation:** Not native; ASIO drivers may support multi-device

### WebAssembly (Future)
- **Web Audio API:** High-level, not realtime-guaranteed
- **AudioWorklet:** Real-time audio thread (requires `audioworklet` feature, nightly Rust, atomics)
- **Not for M0-M3** — document as future platform

---

## OxiDJ Audio Architecture Boundaries

### M0-M3 (Library Only) — What We Need

```rust
// Library crate: oxidj-audio (NO cpal dependency)
pub mod decode {
    pub fn decode_file(path: &Path) -> Result<AudioBuffer<f32>>;  // Symphonia
    pub fn probe_format(path: &Path) -> Result<FormatInfo>;
    pub fn read_metadata(path: &Path) -> Result<TrackMetadata>;
}

pub mod analysis {
    pub fn analyze_bpm(buffer: &AudioBuffer<f32>) -> Result<BpmResult>;
    pub fn analyze_beatgrid(buffer: &AudioBuffer<f32>) -> Result<BeatGrid>;
    pub fn generate_waveform(buffer: &AudioBuffer<f32>, width: u32) -> Vec<f32>;
}

pub mod dsp_offline {
    // Non-realtime DSP: normalization, offline effects, etc.
    pub fn normalize(buffer: &mut AudioBuffer<f32>, target_lufs: f32);
}
```

### M4+ (Realtime Engine) — What We'll Need

```rust
// Separate crate: oxidj-engine (cpal + RT-safe DSP)
pub mod deck {
    pub struct Deck {
        // Resampler (rubato), beat-sync, cue points, looping
    }
}

pub mod mixer {
    pub struct Mixer {
        // Channels, EQ, filter, crossfader, gain
    }
}

pub mod output {
    // CPAL stream management, device selection, buffer sizing
}
```

### Communication: Library → Engine

| Data | Format | Direction |
|------|--------|-----------|
| Decoded audio (full track) | f32 interleaved or planar | Library → Engine (load time) |
| Waveform peaks | `Vec<f32>` (min/max per pixel) | Library → UI |
| Beatgrid | `BeatGrid` struct (BPM, beats, downbeats) | Library → Engine/UI |
| Cue/loop points | `Vec<CuePoint>` | Library ↔ Engine (bidirectional) |
| Analysis metadata | `TrackAnalysis` | Library → Engine |

**Key principle:** Engine never decodes files. Library provides pre-decoded, analyzed assets.

---

## Open Risks & Decisions Needed

| Risk | Impact | Decision Needed |
|------|--------|-----------------|
| **CPAL realtime priority on Linux** | May need `rtkit`/`realtime-dbus` + user config | Document requirements; test on target distros |
| **ASIO on Windows** | Requires proprietary SDK + LLVM/Clang | Make opt-in feature; document build steps |
| **Sample rate conversion quality** | `rubato` has quality/speed tradeoffs | Benchmark `rubato::SincFixedIn` vs `SincInterpolator` |
| **Bluetooth latency** | Unsuitable for DJ monitoring | Document as unsupported; require wired/low-latency |
| **Multi-device output** | Aggregate devices vary by platform | Define engine API for device selection; implement per-platform |
| **Symphonia compile time** | Many feature-gated crates | Profile; consider minimal feature set for M0 |

---

## Evidence Packets for Review

- [ ] CPAL backend matrix + feature flags for OxiDJ targets
- [ ] Symphonia decode benchmark (MP3, FLAC, WAV, MP4) — speed, memory
- [ ] Metadata extraction completeness vs. rekordbox/serato tags
- [ ] Waveform generation quality + performance (500k tracks)
- [ ] Realtime priority setup guide per platform
- [ ] ASIO build documentation (Windows cross-compile from Linux)
- [ ] Rubato resampler quality comparison (sinc vs polynomial)

---

## References

1. RustAudio/cpal — [GitHub](https://github.com/RustAudio/cpal), [README](https://github.com/RustAudio/cpal/blob/master/README.md)
2. DeepWiki CPAL — [Platform Backends](https://deepwiki.com/RustAudio/cpal/3-platform-backends), [Building](https://deepwiki.com/RustAudio/cpal/4-building-and-configuration)
3. Symphonia — [GitHub](https://github.com/jiztastamablastamarang/symphonia), [lib.rs](https://lib.rs/crates/symphonia), [openapps.pro](https://openapps.pro/packages/symphonia)
4. Rodio — [GitHub](https://github.com/julien-leclercq/rodio), [DeepWiki](https://deepwiki.com/RustAudio/rodio)
5. dasp — [crates.io](https://crates.io/crates/dasp)
6. fundsp — [crates.io](https://crates.io/crates/fundsp)
7. rubato — [crates.io](https://crates.io/crates/rubato)
8. valib — [crates.io](https://crates.io/crates/valib)