//! Where rendered blocks go: a CPAL device stream, a silent paced thread
//! (no device / device lost: decks keep moving), or nothing (tests render by
//! hand). Device setup, error classification and teardown all happen
//! outside the callback.

use crate::{Renderer, Shared, Stats};
use cpal::SupportedBufferSize;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
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
        stream: Option<Box<dyn DeviceStream>>,
        returned: rtrb::Consumer<CallbackResources>,
        // Keep callback metadata alive until stream destruction on this thread.
        _state: Arc<CallbackState>,
        kind: OutputKind,
    },
    Silent {
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<Renderer>>,
    },
    Manual,
}

impl Output {
    /// Stop the silent worker and reclaim its renderer on the control thread.
    fn stop_silent(&mut self) -> Option<Renderer> {
        let Self::Silent { stop, thread } = self else {
            return None;
        };
        stop.store(true, Ordering::Release);
        thread.take()?.join().ok()
    }

    /// Close first, then reclaim. In particular, the callback destructor
    /// returns even a renderer that has not seen its first device callback.
    pub(crate) fn reclaim(&mut self) -> Option<Renderer> {
        if let Self::Device {
            stream, returned, ..
        } = self
        {
            drop(stream.take());
            let mut resources = returned.pop().ok()?;
            return resources
                .renderer
                .take()
                .or_else(|| resources.incoming.pop().ok());
        }
        self.stop_silent()
    }

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
                renderer
            })
            .ok();
        Self::Silent { stop, thread }
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        drop(self.reclaim());
    }
}

/// Only the control thread calls this interface. Tests inject stream startup
/// failures here, below the real candidate/renderer handover implementation.
pub(crate) trait DeviceStream {
    fn play(&self) -> Result<(), String>;
}

impl DeviceStream for cpal::Stream {
    fn play(&self) -> Result<(), String> {
        StreamTrait::play(self).map_err(|e| format!("audio output did not start: {e}"))
    }
}

const ARMED: u8 = 1;
const CLOSED: u8 = 2;

#[derive(Default)]
pub(crate) struct CallbackState {
    // A single RMW orders arm against independent callback destruction. The
    // second participant observes the first and marks the live output dead.
    lifecycle: AtomicU8,
    failed: AtomicBool,
}

/// Returned as a unit, including the receiving endpoint: teardown may happen
/// BEFORE control publishes the renderer. Keeping that endpoint alive in the
/// return ring lets control receive its own later publication when reclaiming.
/// Scratch follows it so callback destruction never frees its allocation.
pub(crate) struct CallbackResources {
    renderer: Option<Renderer>,
    incoming: rtrb::Consumer<Renderer>,
    scratch: Vec<f32>,
}

/// A built AND started stream with no renderer yet. A host may invoke data or
/// error callbacks from build/play; these cannot touch decks, queues or stats.
pub(crate) struct Candidate {
    stream: Box<dyn DeviceStream>,
    incoming: rtrb::Producer<Renderer>,
    returned: rtrb::Consumer<CallbackResources>,
    state: Arc<CallbackState>,
    shared: Arc<Shared>,
    kind: OutputKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ArmPhase {
    Published,
    Armed,
}

impl Candidate {
    pub(crate) fn start(
        kind: OutputKind,
        shared: Arc<Shared>,
        build: impl FnOnce(Callback, ErrorCallback) -> Result<Box<dyn DeviceStream>, String>,
    ) -> Result<Self, String> {
        let OutputKind::Device { rate, channels, .. } = kind else {
            unreachable!("only device streams are candidates");
        };
        let (incoming, receive) = rtrb::RingBuffer::new(1);
        let (send_back, returned) = rtrb::RingBuffer::new(1);
        let state = Arc::new(CallbackState::default());
        let channels = usize::from(channels).max(1);
        let step = (SCRATCH / channels).max(1) * channels;
        let callback = Callback {
            resources: Some(CallbackResources {
                renderer: None,
                incoming: receive,
                scratch: vec![0.0; step],
            }),
            returned: send_back,
            state: state.clone(),
            shared: shared.clone(),
            channels,
            rate: u64::from(rate.max(1)),
            origin: Instant::now(),
        };
        let error = ErrorCallback {
            state: state.clone(),
            shared: shared.clone(),
        };
        let stream = build(callback, error)?;
        stream.play()?;
        if state.failed.load(Ordering::Acquire) {
            return Err("audio output failed during startup".into());
        }
        Ok(Self {
            stream,
            incoming,
            returned,
            state,
            shared,
            kind,
        })
    }

    pub(crate) fn rate(&self) -> u32 {
        match self.kind {
            OutputKind::Device { rate, .. } => rate,
            _ => unreachable!(),
        }
    }

    /// Infallible commit: all backend operations already succeeded, the old
    /// worker has joined, and the rate-dependent DSP was prepared off-thread.
    pub(crate) fn arm(self, renderer: Renderer) -> Output {
        self.arm_with(renderer, |_| {})
    }

    // The hook permits deterministic tests to destroy the backend callback
    // on either side of the atomic arm/close handshake, during publication.
    pub(super) fn arm_with(
        mut self,
        renderer: Renderer,
        mut interleave: impl FnMut(ArmPhase),
    ) -> Output {
        self.incoming
            .push(renderer)
            .unwrap_or_else(|_| unreachable!("one renderer per candidate"));
        interleave(ArmPhase::Published);
        let previous = self.state.lifecycle.fetch_or(ARMED, Ordering::AcqRel);
        interleave(ArmPhase::Armed);
        // Catch a fatal error racing the final startup check, without allowing
        // speculative candidate callbacks to mark the silent renderer dead.
        if previous & CLOSED != 0 || self.state.failed.load(Ordering::Acquire) {
            self.shared.stats.dead.store(true, Ordering::Release);
        }
        Output::Device {
            stream: Some(self.stream),
            returned: self.returned,
            _state: self.state,
            kind: self.kind,
        }
    }
}

pub(crate) struct Callback {
    resources: Option<CallbackResources>,
    returned: rtrb::Producer<CallbackResources>,
    state: Arc<CallbackState>,
    shared: Arc<Shared>,
    channels: usize,
    rate: u64,
    origin: Instant,
}

impl Callback {
    pub(crate) fn render<T: cpal::SizedSample + cpal::FromSample<f32>>(
        &mut self,
        out: &mut [T],
        latency: Duration,
    ) {
        let resources = self.resources.as_mut().expect("callback is alive");
        if self.state.lifecycle.load(Ordering::Acquire) & ARMED != 0 && resources.renderer.is_none()
        {
            resources.renderer = resources.incoming.pop().ok();
        }
        let Some(renderer) = resources.renderer.as_mut() else {
            out.fill(T::from_sample_(0.0));
            return;
        };
        let frames = out.len() / self.channels;
        let began = record(&self.shared.stats, frames, self.origin);
        for chunk in out.chunks_mut(resources.scratch.len()) {
            let buffer = &mut resources.scratch[..chunk.len()];
            renderer.render(buffer, self.channels);
            for (o, s) in chunk.iter_mut().zip(buffer.iter()) {
                *o = T::from_sample_(*s);
            }
        }
        finish(
            &self.shared.stats,
            frames,
            self.rate,
            self.origin,
            began,
            latency,
        );
    }
}

impl Drop for Callback {
    fn drop(&mut self) {
        // No allocation/free, lock or wait, even if arm is currently publishing.
        // Return the receiver rather than trying to drain a possibly empty ring.
        self.returned
            .push(self.resources.take().expect("callback is alive"))
            .unwrap_or_else(|_| unreachable!("return slot is empty"));
        if self.state.lifecycle.fetch_or(CLOSED, Ordering::AcqRel) & ARMED != 0 {
            self.shared.stats.dead.store(true, Ordering::Release);
        }
    }
}

pub(crate) struct ErrorCallback {
    state: Arc<CallbackState>,
    shared: Arc<Shared>,
}

impl ErrorCallback {
    pub(crate) fn report(&self, kind: cpal::ErrorKind) {
        let fatal = matches!(
            kind,
            cpal::ErrorKind::DeviceNotAvailable
                | cpal::ErrorKind::HostUnavailable
                | cpal::ErrorKind::StreamInvalidated
        );
        if fatal {
            self.state.failed.store(true, Ordering::Release);
        }
        if self.state.lifecycle.load(Ordering::Acquire) & ARMED == 0 {
            return;
        }
        let stats = &self.shared.stats;
        if kind == cpal::ErrorKind::Xrun {
            stats.xruns.fetch_add(1, Ordering::Relaxed);
        } else {
            stats.errors.fetch_add(1, Ordering::Relaxed);
            if fatal {
                stats.dead.store(true, Ordering::Release);
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
        self.prepare(shared)
            .map(|candidate| candidate.arm(renderer))
    }

    pub(crate) fn prepare(self, shared: Arc<Shared>) -> Result<Candidate, String> {
        use cpal::SampleFormat as F;
        let kind = OutputKind::Device {
            name: self.name.clone(),
            rate: self.rate,
            channels: self.config.channels(),
        };
        Candidate::start(kind, shared, |callback, error| {
            match self.config.sample_format() {
                F::F32 => self.build::<f32>(callback, error),
                F::F64 => self.build::<f64>(callback, error),
                F::I16 => self.build::<i16>(callback, error),
                F::I32 => self.build::<i32>(callback, error),
                F::U16 => self.build::<u16>(callback, error),
                other => Err(format!("unsupported audio sample format {other}")),
            }
        })
    }

    fn build<T>(
        &self,
        mut callback: Callback,
        error: ErrorCallback,
    ) -> Result<Box<dyn DeviceStream>, String>
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        let mut config = self.config.config();
        if let Some(n) = self.buffer_frames {
            config.buffer_size = cpal::BufferSize::Fixed(n);
        }
        let data = move |out: &mut [T], info: &cpal::OutputCallbackInfo| {
            let stamp = info.timestamp();
            callback.render(out, stamp.playback.duration_since(stamp.callback));
        };
        self.device
            .build_output_stream::<T, _, _>(config, data, move |err| error.report(err.kind()), None)
            .map(|stream| Box::new(stream) as Box<dyn DeviceStream>)
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
    fn silent_worker_returns_its_renderer_on_join() {
        let (renderer, mut tx, shared) =
            crate::channel(Default::default(), 48_000, None, true, true);
        let mut output = super::Output::silent(renderer);
        let mut recovered = output.stop_silent().expect("worker joined");
        assert!(std::sync::Arc::ptr_eq(&recovered.shared, &shared));
        assert!(output.stop_silent().is_none());
        tx.push(crate::Command::Limiter(false)).unwrap();
        recovered.render(&mut [0.0; 2], 2);
        assert!(
            !recovered.limiter.on,
            "the original command ring remains connected"
        );
    }

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
