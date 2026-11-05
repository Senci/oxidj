//! Key lock: the engine-independent time-stretch boundary (ADR-008) and its
//! first implementation, Signalsmith Stretch (`signalsmith-stretch =0.1.3`,
//! admitted by the ADR-008 D3 addendum).
//!
//! The trait exposes what ADR-008 requires of any stretch engine: independent
//! tempo (the input/output length ratio of [`TimeStretch::process`]) and
//! pitch ([`TimeStretch::set_transpose`]), latency for frame accounting,
//! reset and pre-roll. Construction allocates and is done off the callback;
//! every other method must be callable from the audio callback.

/// Interleaved-stereo time-stretch engine used by the callback.
pub trait TimeStretch: Send {
    /// `(input, output)` latency in device frames.
    fn latency(&self) -> (usize, usize);
    /// Input frames [`Self::preroll`] wants before the play point.
    fn preroll_frames(&self) -> usize;
    /// Clear all history (no allocation).
    fn reset(&mut self);
    /// Feed `input` (the frames just before the play point) without output.
    fn preroll(&mut self, input: &[f32], tempo: f64);
    /// Consume all of `input`, fill all of `output`: tempo = in / out frames,
    /// pitch unchanged unless [`Self::set_transpose`] says otherwise.
    fn process(&mut self, input: &[f32], output: &mut [f32]);
    /// Pitch multiplier independent of tempo (1.0 = unchanged).
    fn set_transpose(&mut self, factor: f32);
}

/// Signalsmith Stretch, default preset (120 ms block, 30 ms interval).
pub struct Signalsmith {
    inner: signalsmith_stretch::Stretch,
}

impl Signalsmith {
    /// Configure for stereo at `device_rate` and warm the internal buffers
    /// up (allocates; call off the callback).
    pub fn new(device_rate: u32) -> Self {
        let mut inner = signalsmith_stretch::Stretch::preset_default(2, device_rate.max(1));
        // Warm-up: run noise through both tempo extremes so every internal
        // vector reaches its working capacity before the callback uses it.
        let mut seed = 0x2545_f491_u32;
        let noise: Vec<f32> = (0..4096 * 2)
            .map(|_| {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                (seed as f32 / u32::MAX as f32) - 0.5
            })
            .collect();
        let mut out = vec![0.0; 2048 * 2];
        for _ in 0..4 {
            inner.process(&noise[..2048 * 2], &mut out[..1024 * 2]);
            inner.process(&noise, &mut out);
            inner.process(&noise[..1024 * 2], &mut out);
        }
        inner.seek(&noise, 1.0);
        inner.reset();
        Self { inner }
    }
}

impl TimeStretch for Signalsmith {
    fn latency(&self) -> (usize, usize) {
        (self.inner.input_latency(), self.inner.output_latency())
    }

    fn preroll_frames(&self) -> usize {
        // Signalsmith asks for one block plus one interval: 2 × input latency
        // is a block; the interval is a quarter of it (default preset).
        self.inner.input_latency() * 2 + self.inner.input_latency() / 2
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn preroll(&mut self, input: &[f32], tempo: f64) {
        self.inner.seek(input, tempo);
    }

    fn process(&mut self, input: &[f32], output: &mut [f32]) {
        self.inner.process(input, output);
    }

    fn set_transpose(&mut self, factor: f32) {
        if factor.is_finite() && factor > 0.0 {
            self.inner.set_transpose_factor(factor, None);
        }
    }
}
