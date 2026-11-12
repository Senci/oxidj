//! Where rendered blocks go: a CPAL device stream, a silent paced thread
//! (no device / device lost: decks keep moving), or nothing (tests render by
//! hand). Device setup, error classification and teardown all happen
//! outside the callback.

use crate::{Renderer, Shared, Stats};
use cpal::SupportedBufferSize;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Clock of the silent output.
pub(crate) const SILENT_RATE: u32 = 48_000;
/// Frames per silent block (~10.7 ms at 48 kHz).
const SILENT_BLOCK: usize = 512;
/// Callback scratch (samples): rendered in chunks of at most this many.
const SCRATCH: usize = 16_384;

/// Where the engine's audio goes (for the status line).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutputKind {
    Device {
        name: String,
        rate: u32,
        channels: u16,
    },
    /// No device: a paced thread renders into the void.
    Silent,
    /// Tests: blocks are rendered by hand.
    Manual,
}

impl fmt::Display for OutputKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Device {
                name,
                rate,
                channels,
            } => write!(f, "{name} · {rate} Hz · {channels} ch"),
            Self::Silent => f.write_str("silent"),
            Self::Manual => f.write_str("manual"),
        }
    }
}

pub(crate) enum Output {
    Device {
        _stream: cpal::Stream,
        kind: OutputKind,
    },
    Silent {
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    },
    Manual,
}

impl Output {
    pub(crate) fn kind(&self) -> OutputKind {
        match self {
            Self::Device { kind, .. } => kind.clone(),
            Self::Silent { .. } => OutputKind::Silent,
            Self::Manual => OutputKind::Manual,
        }
    }

    /// Render `renderer` on a paced thread into a discarded buffer.
    pub(crate) fn silent(mut renderer: Renderer) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = std::thread::Builder::new()
            .name("oxidj-silent-audio".into())
            .spawn(move || {
                let mut buffer = vec![0.0_f32; SILENT_BLOCK * 2];
                let period = Duration::from_secs_f64(SILENT_BLOCK as f64 / f64::from(SILENT_RATE));
                let mut next = Instant::now();
                while !flag.load(Ordering::Acquire) {
                    renderer.render(&mut buffer, 2);
                    next += period;
                    let now = Instant::now();
                    if next > now {
                        std::thread::sleep(next - now);
                    } else {
                        next = now;
                    }
                }
            })
            .ok();
        Self::Silent { stop, thread }
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        if let Self::Silent { stop, thread } = self {
            stop.store(true, Ordering::Release);
            if let Some(thread) = thread.take() {
                let _ = thread.join();
            }
        }
    }
}

/// Which device channels carry the two buses. Channels are 0-based here
/// (`master: 0` = outputs 1/2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Routing {
    /// First channel of the main stereo pair.
    pub master: u16,
    /// First channel of the headphone cue pair. Silent when the device has
    /// no such pair.
    pub cue: u16,
    /// Split cue for a 2-channel card: main in mono on the left output, cue
    /// in mono on the right (`master` and `cue` are ignored).
    pub split: bool,
}

impl Default for Routing {
    /// Main on outputs 1/2, cue on 3/4.
    fn default() -> Self {
        Self {
            master: 0,
            cue: 2,
            split: false,
        }
    }
}

/// Which output to open and how. Every field is optional; `None` means
/// "what the host suggests". Build one in the UI's settings, or read it from
/// the environment with [`OutputConfig::from_env`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OutputConfig {
    /// cpal host name, case-insensitive (`ALSA`, `JACK`, `ASIO`, `WASAPI`,
    /// `CoreAudio`); `None` prefers JACK on Linux when compiled in and an
    /// automatic output is available, otherwise uses the platform default.
    /// A named device without a host uses the platform default host.
    pub host: Option<String>,
    /// [`OutputInfo::id`] (stable, unique) or the exact device name as listed
    /// by [`list_outputs`] (refused when two devices share it); `None` = the
    /// host's default output.
    pub device: Option<String>,
    /// Device sample rate in Hz; `None` = the device default.
    pub sample_rate: Option<u32>,
    /// Requested callback size in frames (latency); `None` = host default.
    /// Clamped to the device's range; the host may still round it.
    pub buffer_frames: Option<u32>,
    /// Output channels; 4 or more puts headphone cue on 3/4. `None` = the
    /// device default.
    pub channels: Option<u16>,
    /// Which channels carry main and cue.
    pub routing: Routing,
}

impl Routing {
    /// Whether a device with `channels` outputs can carry this routing.
    /// The main pair must exist; a missing cue pair only silences cue.
    pub(crate) fn check(self, channels: u16) -> Result<(), String> {
        if self.split {
            return if channels >= 2 {
                Ok(())
            } else {
                Err("split cue needs a 2-channel output".to_owned())
            };
        }
        if self.master.abs_diff(self.cue) < 2 {
            return Err(format!(
                "main outputs {}/{} and cue outputs {}/{} overlap",
                self.master + 1,
                self.master + 2,
                self.cue + 1,
                self.cue + 2
            ));
        }
        if channels > 1 && u32::from(self.master) + 2 > u32::from(channels) {
            return Err(format!(
                "main outputs {}/{} do not exist on a {channels}-channel output",
                self.master + 1,
                self.master + 2
            ));
        }
        Ok(())
    }
}

impl OutputConfig {
    /// `OXIDJ_AUDIO_HOST`, `OXIDJ_AUDIO_DEVICE`, `OXIDJ_AUDIO_RATE`,
    /// `OXIDJ_AUDIO_BUFFER`, `OXIDJ_AUDIO_CHANNELS`; routing with
    /// `OXIDJ_AUDIO_MASTER` and `OXIDJ_AUDIO_CUE` (first output of the pair,
    /// counted from 1 as on the sound card) and `OXIDJ_AUDIO_SPLIT_CUE=1`
    /// (which asks for 2 channels). A named device without
    /// rate or channels asks for 4 channels at 48 kHz (master + cue), as the
    /// cue smoke always did.
    pub fn from_env() -> Self {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// [`OutputConfig::from_env`] over any key lookup.
    pub(crate) fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Self {
        let text = |k: &str| get(k).filter(|v| !v.trim().is_empty());
        let number = |k: &str| text(k).and_then(|v| v.trim().parse::<u32>().ok());
        let device = text("OXIDJ_AUDIO_DEVICE");
        let named = device.is_some();
        // Outputs are numbered from 1 on the card, from 0 in `Routing`.
        let output = |k: &str, default: u16| {
            number(k)
                .and_then(|n| u16::try_from(n.checked_sub(1)?).ok())
                .unwrap_or(default)
        };
        let split = text("OXIDJ_AUDIO_SPLIT_CUE").is_some_and(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        });
        Self {
            host: text("OXIDJ_AUDIO_HOST"),
            device,
            sample_rate: number("OXIDJ_AUDIO_RATE").or(named.then_some(48_000)),
            buffer_frames: number("OXIDJ_AUDIO_BUFFER"),
            channels: number("OXIDJ_AUDIO_CHANNELS")
                .and_then(|c| u16::try_from(c).ok())
                .or(split.then_some(2))
                .or(named.then_some(4)),
            routing: Routing {
                master: output("OXIDJ_AUDIO_MASTER", 0),
                cue: output("OXIDJ_AUDIO_CUE", 2),
                split,
            },
        }
    }
}

/// One output device as the settings screen lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputInfo {
    pub host: String,
    /// Human-readable name (hosts such as ALSA repeat it across endpoints).
    pub device: String,
    /// Stable id, unique within the host; pass it as [`OutputConfig::device`].
    pub id: String,
    /// The host's default output.
    pub default: bool,
    /// Largest channel count the device offers.
    pub max_channels: u16,
    /// Lowest and highest supported sample rate (Hz).
    pub rates: (u32, u32),
    /// Supported callback sizes in frames, when the host reports them.
    pub buffer_frames: Option<(u32, u32)>,
}

/// Every output of every compiled-in host. Enumerates devices: call it from
/// the UI thread when the settings screen opens, never per frame.
pub fn list_outputs() -> Vec<OutputInfo> {
    let mut found = Vec::new();
    for id in cpal::available_hosts() {
        let Ok(host) = cpal::host_from_id(id) else {
            continue;
        };
        let default = host.default_output_device().and_then(|d| name_of(&d));
        let Ok(devices) = host.output_devices() else {
            continue;
        };
        for device in devices {
            let (Some(name), Ok(configs)) = (name_of(&device), device.supported_output_configs())
            else {
                continue;
            };
            let mut info = OutputInfo {
                host: id.name().to_owned(),
                default: default.as_deref() == Some(name.as_str()),
                device: name,
                id: id_of(&device).unwrap_or_default(),
                max_channels: 0,
                rates: (u32::MAX, 0),
                buffer_frames: None,
            };
            for c in configs {
                info.max_channels = info.max_channels.max(c.channels());
                info.rates.0 = info.rates.0.min(c.min_sample_rate());
                info.rates.1 = info.rates.1.max(c.max_sample_rate());
                if let SupportedBufferSize::Range { min, max } = *c.buffer_size() {
                    let (lo, hi) = info.buffer_frames.unwrap_or((min, max));
                    info.buffer_frames = Some((lo.min(min), hi.max(max)));
                }
            }
            if info.max_channels > 0 {
                found.push(info);
            }
        }
    }
    found
}

fn name_of(device: &cpal::Device) -> Option<String> {
    device.description().ok().map(|d| d.name().to_owned())
}

fn id_of(device: &cpal::Device) -> Option<String> {
    device.id().ok().map(|id| id.id().to_owned())
}

fn usable(format: cpal::SampleFormat) -> bool {
    use cpal::SampleFormat as F;
    matches!(format, F::F32 | F::F64 | F::I16 | F::I32 | F::U16)
}

fn select_automatic_host<T>(
    automatic_device: bool,
    jack: impl FnOnce() -> Option<T>,
    default: impl FnOnce() -> T,
) -> T {
    if automatic_device && let Some(host) = jack() {
        return host;
    }
    default()
}

/// Prefer JACK only for automatic device selection, and fall back if absent.
fn preferred_host(automatic_device: bool) -> cpal::Host {
    #[cfg(all(target_os = "linux", feature = "jack"))]
    let jack = || {
        let id = cpal::available_hosts()
            .into_iter()
            .find(|id| id.name().eq_ignore_ascii_case("jack"))?;
        let host = cpal::host_from_id(id).ok()?;
        host.default_output_device()?;
        Some(host)
    };
    #[cfg(not(all(target_os = "linux", feature = "jack")))]
    let jack = || None;
    select_automatic_host(automatic_device, jack, cpal::default_host)
}

pub(crate) fn select_host(want: &OutputConfig) -> Result<(cpal::Host, bool), String> {
    if let Some(name) = want.host.as_deref() {
        let id = cpal::available_hosts()
            .into_iter()
            .find(|id| id.name().eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("audio host not available: {name}"))?;
        let host = cpal::host_from_id(id).map_err(|e| format!("audio host {name} failed: {e}"))?;
        return Ok((host, false));
    }
    let host = preferred_host(want.device.is_none());
    let fallback = want.device.is_none()
        && host.id().name().eq_ignore_ascii_case("jack")
        && host.id() != cpal::default_host().id();
    Ok((host, fallback))
}

/// An output device and the stream configuration chosen for it.
pub(crate) struct Device {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    name: String,
    pub(crate) rate: u32,
    buffer_frames: Option<u32>,
}

impl Device {
    pub(crate) fn open_on(want: &OutputConfig, host: &cpal::Host) -> Result<Self, String> {
        // Exact endpoint selection is opt-in and never changes the OS default.
        let device = if let Some(name) = want.device.as_deref() {
            let mut by_id = None;
            let mut by_name = Vec::new();
            for d in host
                .output_devices()
                .map_err(|e| format!("cannot enumerate audio outputs: {e}"))?
            {
                if id_of(&d).as_deref() == Some(name) {
                    by_id = Some(d);
                    break;
                }
                if name_of(&d).as_deref() == Some(name) {
                    by_name.push(d);
                }
            }
            match (by_id, by_name.len()) {
                (Some(device), _) => device,
                (None, 0) => return Err(format!("audio output not found: {name}")),
                (None, 1) => by_name.remove(0),
                (None, _) => return Err(format!("audio output name is ambiguous: {name}")),
            }
        } else {
            host.default_output_device()
                .ok_or_else(|| "no audio output device".to_owned())?
        };
        let default = device
            .default_output_config()
            .map_err(|e| format!("audio output unavailable: {e}"));
        let config = if want.sample_rate.is_none() && want.channels.is_none() {
            default?
        } else {
            // Fail closed: a requested channel count (cue pair) or rate that
            // the device lacks is an error, never a silent downgrade.
            let default = default.ok();
            let channels = want
                .channels
                .or(default.as_ref().map(cpal::SupportedStreamConfig::channels));
            let rate = want
                .sample_rate
                .or(default
                    .as_ref()
                    .map(cpal::SupportedStreamConfig::sample_rate))
                .unwrap_or(48_000);
            device
                .supported_output_configs()
                .map_err(|e| format!("audio output unavailable: {e}"))?
                .filter(|c| {
                    channels.is_none_or(|n| c.channels() == n)
                        && c.contains_rate(rate)
                        && usable(c.sample_format())
                })
                // f32 first: no conversion in the callback.
                .min_by_key(|c| c.sample_format() != cpal::SampleFormat::F32)
                .ok_or_else(|| match channels {
                    Some(n) => format!(
                        "audio output does not offer {n} channels at {rate} Hz in a supported sample format"
                    ),
                    None => format!(
                        "audio output does not offer {rate} Hz in a supported sample format"
                    ),
                })?
                .with_sample_rate(rate)
        };
        want.routing.check(config.channels())?;
        let buffer_frames = want.buffer_frames.map(|n| match *config.buffer_size() {
            SupportedBufferSize::Range { min, max } => n.clamp(min, max.max(min)),
            SupportedBufferSize::Unknown => n,
        });
        let name = name_of(&device).unwrap_or_else(|| "default output".to_owned());
        Ok(Self {
            rate: config.sample_rate(),
            device,
            config,
            name,
            buffer_frames,
        })
    }

    pub(crate) fn play(self, renderer: Renderer, shared: Arc<Shared>) -> Result<Output, String> {
        use cpal::SampleFormat as F;
        let stream = match self.config.sample_format() {
            F::F32 => self.build::<f32>(renderer, shared),
            F::F64 => self.build::<f64>(renderer, shared),
            F::I16 => self.build::<i16>(renderer, shared),
            F::I32 => self.build::<i32>(renderer, shared),
            F::U16 => self.build::<u16>(renderer, shared),
            other => Err(format!("unsupported audio sample format {other}")),
        }?;
        stream
            .play()
            .map_err(|e| format!("audio output did not start: {e}"))?;
        Ok(Output::Device {
            _stream: stream,
            kind: OutputKind::Device {
                name: self.name,
                rate: self.rate,
                channels: self.config.channels(),
            },
        })
    }

    fn build<T>(&self, mut renderer: Renderer, shared: Arc<Shared>) -> Result<cpal::Stream, String>
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        let mut config = self.config.config();
        if let Some(n) = self.buffer_frames {
            config.buffer_size = cpal::BufferSize::Fixed(n);
        }
        let channels = usize::from(config.channels).max(1);
        let step = (SCRATCH / channels).max(1) * channels;
        // Allocated here, before the stream exists; the callback only reuses it.
        let mut scratch = vec![0.0_f32; step];
        let origin = Instant::now();
        let rate = u64::from(self.rate.max(1));
        let stats = shared.clone();
        let data = move |out: &mut [T], info: &cpal::OutputCallbackInfo| {
            let frames = out.len() / channels;
            let began = record(&stats.stats, frames, origin);
            for chunk in out.chunks_mut(step) {
                let Some(buffer) = scratch.get_mut(..chunk.len()) else {
                    return;
                };
                renderer.render(buffer, channels);
                for (o, s) in chunk.iter_mut().zip(buffer.iter()) {
                    *o = T::from_sample_(*s);
                }
            }
            let stamp = info.timestamp();
            let latency = stamp.playback.duration_since(stamp.callback);
            finish(&stats.stats, frames, rate, origin, began, latency);
        };
        let error = move |err: cpal::Error| {
            let stats = &shared.stats;
            match err.kind() {
                cpal::ErrorKind::Xrun => {
                    stats.xruns.fetch_add(1, Ordering::Relaxed);
                }
                cpal::ErrorKind::DeviceNotAvailable
                | cpal::ErrorKind::HostUnavailable
                | cpal::ErrorKind::StreamInvalidated => {
                    stats.errors.fetch_add(1, Ordering::Relaxed);
                    stats.dead.store(true, Ordering::Release);
                }
                _ => {
                    stats.errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        };
        self.device
            .build_output_stream::<T, _, _>(config, data, error, None)
            .map_err(|e| format!("audio output could not open: {e}"))
    }
}

fn nanos(origin: Instant) -> u64 {
    u64::try_from(origin.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

/// Callback timing at entry (atomics only); returns the entry time.
fn record(stats: &Stats, frames: usize, origin: Instant) -> u64 {
    let now = nanos(origin);
    let last = stats.last_ns.swap(now, Ordering::Relaxed);
    if last != 0 {
        stats
            .max_interval_ns
            .fetch_max(now.saturating_sub(last), Ordering::Relaxed);
    }
    let frames = u32::try_from(frames).unwrap_or(u32::MAX);
    stats.callbacks.fetch_add(1, Ordering::Relaxed);
    stats.frames.fetch_add(u64::from(frames), Ordering::Relaxed);
    stats.min_block.fetch_min(frames, Ordering::Relaxed);
    stats.max_block.fetch_max(frames, Ordering::Relaxed);
    stats.block.store(frames, Ordering::Relaxed);
    now
}

/// Callback timing at exit (atomics only): how long the block took to
/// render, that as a share of the block's own duration, and how far ahead
/// of the DAC the block was written.
fn finish(stats: &Stats, frames: usize, rate: u64, origin: Instant, began: u64, latency: Duration) {
    let spent = nanos(origin).saturating_sub(began);
    stats.max_render_ns.fetch_max(spent, Ordering::Relaxed);
    stats.render_ns.fetch_add(spent, Ordering::Relaxed);
    let block_ns = (frames as u64).saturating_mul(1_000_000_000) / rate;
    if let Some(load) = spent.saturating_mul(1000).checked_div(block_ns) {
        stats
            .max_load_permille
            .fetch_max(u32::try_from(load).unwrap_or(u32::MAX), Ordering::Relaxed);
    }
    stats.latency_ns.store(
        u64::try_from(latency.as_nanos()).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
}

#[cfg(test)]
mod host_selection_tests {
    use super::select_automatic_host;

    #[test]
    fn automatic_selection_prefers_available_jack_and_falls_back() {
        assert_eq!(
            select_automatic_host(true, || Some("jack"), || "default"),
            "jack"
        );
        assert_eq!(
            select_automatic_host(true, || None, || "default"),
            "default"
        );
        assert_eq!(
            select_automatic_host(
                false,
                || panic!("explicit device must not probe JACK"),
                || "default"
            ),
            "default"
        );
    }
}
