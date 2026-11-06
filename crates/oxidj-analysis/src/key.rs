//! Offline musical-key suggestion from chroma energy.
//!
//! This compact estimator accumulates Goertzel energy for equal-tempered pitch
//! bins and compares the resulting chroma vector with Krumhansl-Schmuckler
//! major/minor profiles. It is a suggestion, never a replacement for imported
//! vendor metadata. Unsupported, silent, non-finite, or ambiguous input returns
//! `None` rather than inventing a key.

const FRAME: usize = 4096;
const FIRST_MIDI: i32 = 36;
const LAST_MIDI: i32 = 83;
const MIN_ENERGY: f64 = 1e-8;
const MIN_SCORE: f64 = 0.25;
const MIN_MARGIN: f64 = 0.025;

/// A twelve-tone equal-tempered tonic; C = 0 through B = 11.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// Major tonality, tonic pitch class C=0..B=11.
    Major(u8),
    /// Minor tonality, tonic pitch class C=0..B=11.
    Minor(u8),
}

/// Streaming PCM-to-key estimator. It retains at most one analysis frame.
#[derive(Clone, Debug)]
pub struct KeyBuilder {
    sample_rate: u32,
    frame: Vec<f32>,
    chroma: [f64; 12],
    invalid: bool,
}

impl KeyBuilder {
    /// Create an estimator; sample rates below 8 kHz fail closed at `finish`.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            frame: Vec::with_capacity(FRAME),
            chroma: [0.0; 12],
            invalid: false,
        }
    }

    /// Add mono samples. Any non-finite value invalidates the whole result.
    pub fn push(&mut self, samples: &[f32]) {
        if self.invalid {
            return;
        }
        if samples.iter().any(|sample| !sample.is_finite()) {
            self.invalid = true;
            self.frame.clear();
            self.chroma = [0.0; 12];
            return;
        }
        for &sample in samples {
            self.frame.push(sample);
            if self.frame.len() == FRAME {
                self.consume_frame();
            }
        }
    }

    /// Finish and return the best unambiguous major/minor key suggestion.
    pub fn finish(mut self) -> Option<Key> {
        if self.invalid || self.sample_rate < 8_000 {
            return None;
        }
        if self.frame.len() >= FRAME / 2 {
            self.consume_frame();
        }
        let total: f64 = self.chroma.iter().sum();
        if !total.is_finite() || total < MIN_ENERGY {
            return None;
        }
        let chroma = normalize(&self.chroma)?;
        let mut ranked = Vec::with_capacity(24);
        for tonic in 0..12 {
            ranked.push((
                correlation(&chroma, &rotate(&MAJOR, tonic)),
                Key::Major(tonic as u8),
            ));
            ranked.push((
                correlation(&chroma, &rotate(&MINOR, tonic)),
                Key::Minor(tonic as u8),
            ));
        }
        ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
        let (best, key) = ranked[0];
        (best.is_finite() && best >= MIN_SCORE && best - ranked[1].0 >= MIN_MARGIN).then_some(key)
    }

    fn consume_frame(&mut self) {
        if self.frame.len() < FRAME / 2 {
            return;
        }
        let n = self.frame.len();
        let mut local = [0.0f64; 12];
        for midi in FIRST_MIDI..=LAST_MIDI {
            let hz = 440.0 * 2.0f64.powf((f64::from(midi) - 69.0) / 12.0);
            let omega = core::f64::consts::TAU * hz / f64::from(self.sample_rate);
            let coeff = 2.0 * omega.cos();
            let (mut s1, mut s2) = (0.0f64, 0.0f64);
            for (i, sample) in self.frame.iter().enumerate() {
                let window = if n > 1 {
                    0.5 - 0.5 * (core::f64::consts::TAU * i as f64 / (n - 1) as f64).cos()
                } else {
                    1.0
                };
                let s = f64::from(*sample) * window + coeff * s1 - s2;
                s2 = s1;
                s1 = s;
            }
            let energy = (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(0.0);
            local[midi.rem_euclid(12) as usize] += energy;
        }
        for (total, energy) in self.chroma.iter_mut().zip(local) {
            *total += energy;
        }
        self.frame.clear();
    }
}

const MAJOR: [f64; 12] = [
    6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
];
const MINOR: [f64; 12] = [
    6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
];

fn rotate(profile: &[f64; 12], tonic: usize) -> [f64; 12] {
    core::array::from_fn(|pitch| profile[(pitch + 12 - tonic) % 12])
}

fn normalize(values: &[f64; 12]) -> Option<[f64; 12]> {
    let mean = values.iter().sum::<f64>() / 12.0;
    let centered: [f64; 12] = core::array::from_fn(|i| values[i] - mean);
    let norm = centered.iter().map(|v| v * v).sum::<f64>().sqrt();
    (norm.is_finite() && norm > 0.0).then(|| centered.map(|v| v / norm))
}

fn correlation(chroma: &[f64; 12], profile: &[f64; 12]) -> f64 {
    let mean = profile.iter().sum::<f64>() / 12.0;
    let centered: [f64; 12] = core::array::from_fn(|i| profile[i] - mean);
    let norm = centered.iter().map(|v| v * v).sum::<f64>().sqrt();
    chroma.iter().zip(centered).map(|(a, b)| a * b).sum::<f64>() / norm
}
