//! Where rendered blocks go: a CPAL device stream, a silent paced thread
//! (no device / device lost: decks keep moving), or nothing (tests render by
//! hand). Device setup, error classification and teardown all happen
//! outside the callback.

use crate::{Renderer, Shared, Stats};
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

/// The default output device and its default configuration.
pub(crate) struct Device {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    name: String,
    pub(crate) rate: u32,
}

impl Device {
    pub(crate) fn open() -> Result<Self, String> {
        let host = cpal::default_host();
        // Exact endpoint selection is opt-in and never changes the OS default.
        // A named cue endpoint must support four channels; fail closed rather
        // than leaking cue into a stereo/default output.
        let selected = std::env::var("OXIDJ_AUDIO_DEVICE").ok();
        let device = if let Some(name) = selected.as_deref() {
            let mut matches = host
                .output_devices()
                .map_err(|e| format!("cannot enumerate audio outputs: {e}"))?
                .filter(|d| d.description().is_ok_and(|d| d.name() == name));
            let device = matches
                .next()
                .ok_or_else(|| format!("audio output not found: {name}"))?;
            if matches.next().is_some() {
                return Err(format!("audio output name is ambiguous: {name}"));
            }
            device
        } else {
            host.default_output_device()
                .ok_or_else(|| "no audio output device".to_owned())?
        };
        let config = if selected.is_some() {
            device
                .supported_output_configs()
                .map_err(|e| format!("audio output unavailable: {e}"))?
                .find(|c| {
                    c.channels() == 4
                        && c.min_sample_rate() <= 48_000
                        && c.max_sample_rate() >= 48_000
                        && matches!(
                            c.sample_format(),
                            cpal::SampleFormat::F32
                                | cpal::SampleFormat::F64
                                | cpal::SampleFormat::I16
                                | cpal::SampleFormat::I32
                                | cpal::SampleFormat::U16
                        )
                })
                .ok_or_else(|| {
                    "selected headphone output requires 4 channels at 48000 Hz".to_owned()
                })?
                .with_sample_rate(48_000)
        } else {
            device
                .default_output_config()
                .map_err(|e| format!("audio output unavailable: {e}"))?
        };
        let name = device
            .description()
            .map(|d| d.name().to_owned())
            .unwrap_or_else(|_| "default output".to_owned());
        Ok(Self {
            rate: config.sample_rate(),
            device,
            config,
            name,
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
        let config = self.config.config();
        let channels = usize::from(config.channels).max(1);
        let step = (SCRATCH / channels).max(1) * channels;
        // Allocated here, before the stream exists; the callback only reuses it.
        let mut scratch = vec![0.0_f32; step];
        let origin = Instant::now();
        let stats = shared.clone();
        let data = move |out: &mut [T], _: &cpal::OutputCallbackInfo| {
            record(&stats.stats, out.len() / channels, origin);
            for chunk in out.chunks_mut(step) {
                let Some(buffer) = scratch.get_mut(..chunk.len()) else {
                    return;
                };
                renderer.render(buffer, channels);
                for (o, s) in chunk.iter_mut().zip(buffer.iter()) {
                    *o = T::from_sample_(*s);
                }
            }
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

/// Callback timing (atomics only).
fn record(stats: &Stats, frames: usize, origin: Instant) {
    let now = u64::try_from(origin.elapsed().as_nanos()).unwrap_or(u64::MAX);
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
}
