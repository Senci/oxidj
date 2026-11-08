//! Bounded synthetic step-only inspection. Policy v1 uses taktus' 100 Hz
//! squared nonnegative spectral-rise envelope, local maxima above 20% of its
//! maximum, 80 ms nonmaximum suppression,
//! and the engine's 22 ms latency correction plus the 5 ms onset-cell centre
//! applied to actual frame time.
//! Fits are least-squares pulse ordinals, never octave-folded. No PCM is retained.
use oxidj_core::step_tempo::{
    ComputedStepTempoV1, StepEvidence, StepMap, StepRefusal, StepSegment,
};
use taktus::{Ear, tempo_of};

pub use oxidj_core::step_tempo::{STEP_ALGORITHM, STEP_POLICY};
pub const MAX_ONSETS: usize = 18_000;
pub const MAX_CANDIDATES: usize = 4096;
pub const MAX_PEAKS: usize = 2250;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceLimit;
impl std::fmt::Display for ResourceLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("step-tempo ResourceLimit")
    }
}
impl std::error::Error for ResourceLimit {}

pub struct StepTempoBuilder {
    rate: u32,
    frames: u64,
    ear: Option<Ear>,
    onsets: Vec<f32>,
    first_time: f64,
    invalid: bool,
    capped: bool,
}
impl StepTempoBuilder {
    pub fn new(rate: u32) -> Self {
        Self {
            rate,
            frames: 0,
            ear: (8000..=48000).contains(&rate).then(|| Ear::new(rate)),
            onsets: Vec::with_capacity(MAX_ONSETS),
            first_time: 0.0,
            invalid: false,
            capped: false,
        }
    }
    pub fn push(&mut self, samples: &[f32]) {
        for &sample in samples {
            self.frames = match self.frames.checked_add(1) {
                Some(n) => n,
                None => {
                    self.capped = true;
                    u64::MAX
                }
            };
            if self.frames > u64::from(self.rate) * 180 {
                self.capped = true;
            }
            if !sample.is_finite() {
                self.invalid = true;
            }
            if self.capped || self.invalid {
                continue;
            }
            if let Some(ear) = &mut self.ear
                && let Some(onset) = ear.hear(sample)
            {
                if self.onsets.is_empty() {
                    // Quantized maxima denote a 10 ms onset cell. Use its
                    // centre, not its lower boundary, after engine latency.
                    self.first_time = self.frames as f64 / f64::from(self.rate) - 0.022 + 0.005;
                }
                if self.onsets.len() == MAX_ONSETS {
                    self.capped = true;
                } else {
                    self.onsets.push(onset * onset);
                }
            }
        }
    }
    pub fn buffered_onsets(&self) -> usize {
        self.onsets.len()
    }
    pub fn resource_limited(&self) -> bool {
        self.ear.is_some() && self.capped
    }
    /// Conservative live heap bound including engine scratch: < 1 MiB, below 8 MiB.
    pub const fn heap_bound_bytes() -> usize {
        // Envelope + conservative 32 bytes/bin for admitted taktus scratch,
        // peaks + matched peaks, at most 64 pulse fits and fixed overhead.
        MAX_ONSETS * 36 + MAX_PEAKS * 16 + 64 * core::mem::size_of::<Fit>() + 65_536
    }
    pub fn finish(self) -> Result<ComputedStepTempoV1, ResourceLimit> {
        use StepRefusal::*;
        let no = |reason| Ok(ComputedStepTempoV1::NoMap(reason));
        if self.ear.is_none() {
            return no(UnsupportedRate);
        }
        if self.capped {
            return Err(ResourceLimit);
        }
        if self.invalid {
            return no(NonFiniteInput);
        }
        if self.frames < u64::from(self.rate) * 8 {
            return no(Insufficient);
        }
        if self.onsets.iter().any(|x| !x.is_finite()) {
            return no(UnsupportedModel);
        }
        let max = self.onsets.iter().copied().fold(0.0f32, f32::max);
        if max <= 1e-6 {
            return no(Silent);
        }
        let mut peaks: Vec<f64> = Vec::with_capacity(MAX_PEAKS);
        for i in 1..self.onsets.len().saturating_sub(1) {
            let x = self.onsets[i];
            if x >= max * 0.2 && x > self.onsets[i - 1] && x >= self.onsets[i + 1] {
                let t = self.first_time + i as f64 * 0.01;
                if peaks.last().is_none_or(|last| t - last >= 0.08) {
                    if peaks.len() == MAX_PEAKS {
                        return Err(ResourceLimit);
                    }
                    peaks.push(t);
                }
            }
        }
        if peaks.len() < 13 {
            return no(Silent);
        }
        let end = self.frames as f64 / f64::from(self.rate);
        let mut budget = Budget(0);
        // Competing whole-span pulse trains are considered before energy rejection.
        let mut hypotheses: Vec<Fit> = Vec::with_capacity(64);
        for a in 0..peaks.len().min(8) {
            for b in a + 1..(a + 5).min(peaks.len()) {
                let period = peaks[b] - peaks[a];
                if !(0.375..=0.6).contains(&period) {
                    continue;
                }
                budget.tick()?;
                let fit = pulse_fit(&peaks, peaks[a], period, end, &self.onsets, self.first_time);
                if fit.coverage >= 0.9
                    && fit.residual <= 0.03
                    && end - fit.anchor >= 8.0
                    && hypotheses.iter().all(|old| {
                        (old.period - fit.period).abs() > 0.001
                            || phase_distance(old.anchor, fit.anchor, period) > 0.03
                    })
                {
                    hypotheses.push(fit);
                }
            }
        }
        if hypotheses.len() > 1
            && tempo_of(&self.onsets)
                .is_none_or(|b| !b.sure.is_finite() || b.sure < 0.05 || !b.catch.is_finite())
        {
            hypotheses.clear();
        }
        for (i, a) in hypotheses.iter().enumerate() {
            for b in &hypotheses[i + 1..] {
                if (a.coverage - b.coverage).abs() <= 0.05
                    && (a.residual - b.residual).abs() <= 0.005
                    // A competing full-support phase at the same tempo is
                    // conservatively ambiguous even when overlapping decays
                    // bias spectral onset energy towards one phase. Never use
                    // that detector artefact to select a rhythmic phase.
                    && ((a.unexplained - b.unexplained).abs() <= 0.05
                        || (a.period-b.period).abs()/a.period.min(b.period)<=0.01)
                    && ((a.period - b.period).abs() / a.period.min(b.period) > 0.01
                        || phase_distance(a.anchor, b.anchor, a.period) > 0.03)
                {
                    return no(Ambiguous);
                }
            }
        }
        // Split the worst linear run at the least-squares change point. Every
        // proposed slice fit consumes the fixed candidate budget, including failures.
        let mut ranges = vec![(0, peaks.len() - 1)];
        loop {
            let mut worst = None;
            for (i, &(a, b)) in ranges.iter().enumerate() {
                let f = linear_fit(&peaks, a, b, &mut budget)?;
                if f.residual > 0.03 && worst.is_none_or(|(_, r)| f.residual > r) {
                    worst = Some((i, f.residual));
                }
            }
            let Some((i, _)) = worst else {
                break;
            };
            if ranges.len() >= 4 {
                return no(UnsupportedModel);
            }
            let (a, b) = ranges[i];
            let mut best = None;
            for cut in a + 3..b.saturating_sub(2) {
                let l = linear_fit(&peaks, a, cut, &mut budget)?;
                let r = linear_fit(&peaks, cut, b, &mut budget)?;
                let cost = l.sse + r.sse;
                if best.is_none_or(|(_, v)| cost < v) {
                    best = Some((cut, cost));
                }
            }
            let Some((cut, _)) = best else {
                return no(UnsupportedModel);
            };
            ranges.splice(i..=i, [(a, cut), (cut, b)]);
        }
        let mut i = 0;
        while i + 1 < ranges.len() {
            let merged = linear_fit(&peaks, ranges[i].0, ranges[i + 1].1, &mut budget)?;
            if merged.residual <= 0.03 {
                let end = ranges[i + 1].1;
                ranges[i].1 = end;
                ranges.remove(i + 1);
            } else {
                i += 1;
            }
        }
        if ranges.len() > 3 {
            return no(UnsupportedModel);
        }
        let segments = match self.fit_ranges(&peaks, &ranges, &mut budget, false)? {
            Ok(s) => s,
            Err(reason) => return no(reason),
        };
        // Challenge neighboring beat-aligned change hypotheses, sharing the
        // very same evidence gates, but before the unexplained-energy cutoff.
        for boundary in 1..ranges.len() {
            for delta in [-2isize, -1, 1, 2] {
                let cut = ranges[boundary].0 as isize + delta;
                if cut <= ranges[boundary - 1].0 as isize || cut >= ranges[boundary].1 as isize {
                    continue;
                }
                let mut alternate = ranges.clone();
                alternate[boundary - 1].1 = cut as usize;
                alternate[boundary].0 = cut as usize;
                let Ok(other) = self.fit_ranges(&peaks, &alternate, &mut budget, true)? else {
                    continue;
                };
                let comparable = segments.iter().zip(&other).all(|(a, b)| {
                    (a.evidence.supported_fraction - b.evidence.supported_fraction).abs() <= 0.05
                        && (a.evidence.max_residual_ms - b.evidence.max_residual_ms).abs() <= 5.0
                        && (a.evidence.unexplained_fraction - b.evidence.unexplained_fraction).abs()
                            <= 0.05
                });
                if comparable
                    && segments.iter().zip(&other).any(|(a, b)| {
                        a.start_ms.abs_diff(b.start_ms) > 30
                            || (a.bpm - b.bpm).abs() / a.bpm.min(b.bpm) > 0.01
                    })
                {
                    return no(Ambiguous);
                }
            }
        }

        let map = match StepMap::new(self.rate, self.frames, segments) {
            Ok(map) => map,
            Err(_) => return no(UnsupportedModel),
        };
        Ok(ComputedStepTempoV1::from_map(map))
    }
    fn fit_ranges(
        &self,
        peaks: &[f64],
        ranges: &[(usize, usize)],
        budget: &mut Budget,
        ambiguity_candidate: bool,
    ) -> Result<Result<Vec<StepSegment>, StepRefusal>, ResourceLimit> {
        use StepRefusal::*;
        let no = |reason| Ok(Err(reason));
        let end = self.frames as f64 / f64::from(self.rate);
        let mut segments = Vec::with_capacity(3);
        let mut previous: Option<(f64, f64, usize)> = None;
        for &(a, b) in ranges {
            let fit = linear_fit(peaks, a, b, budget)?;
            let bpm = 60.0 / fit.period;
            let stop = if b == peaks.len() - 1 { end } else { peaks[b] };
            if stop - fit.anchor < 8.0 || b - a < 12 {
                return no(ShortSegment);
            }
            if !(100.0..=160.0).contains(&bpm) {
                return no(UnsupportedModel);
            }
            if let Some((anchor, period, ordinal)) = previous {
                if (fit.anchor - (anchor + (a - ordinal) as f64 * period)).abs() > 0.001 {
                    return no(UnsupportedModel);
                }
                let diff = (period - fit.period).abs() / period.min(fit.period);
                if diff < 0.05 {
                    return no(UnsupportedModel);
                }
            }
            let lo = ((fit.anchor - self.first_time) * 100.0).max(0.0) as usize;
            let hi = ((stop - self.first_time) * 100.0).max(0.0) as usize;
            let envelope = &self.onsets[lo.min(self.onsets.len())..hi.min(self.onsets.len())];
            let Some(beat) = tempo_of(envelope) else {
                return no(Silent);
            };
            if !beat.sure.is_finite() || beat.sure < 0.05 || !beat.catch.is_finite() {
                return no(Silent);
            }
            // One-to-one matching; a late unsupported tail cannot inherit a
            // perfect score from the earlier fitted onsets.
            let predicted = ((stop - fit.anchor) / fit.period - 1e-9).ceil() as usize;
            let mut matched = 0usize;
            let mut residual = 0.0f64;
            let mut used = Vec::with_capacity(b - a + 1);
            let mut cursor = a;
            for n in 0..predicted {
                let due = fit.anchor + n as f64 * fit.period;
                while cursor <= b && peaks[cursor] < due - 0.03 {
                    cursor += 1;
                }
                if cursor <= b && (peaks[cursor] - due).abs() <= 0.03 {
                    residual = residual.max((peaks[cursor] - due).abs());
                    used.push(peaks[cursor]);
                    matched += 1;
                    cursor += 1;
                }
            }
            let coverage = matched as f64 / predicted.max(1) as f64;
            if coverage < 0.9 {
                return no(UnsupportedModel);
            }
            let energy: f64 = envelope.iter().map(|&v| f64::from(v.max(0.0))).sum();
            let unexplained: f64 = envelope
                .iter()
                .enumerate()
                .filter(|(j, _)| {
                    let t = self.first_time + (lo + *j) as f64 * 0.01;
                    used.iter().all(|p| (t - p).abs() > 0.030000001)
                })
                .map(|(_, v)| f64::from(v.max(0.0)))
                .sum();
            let fraction = if energy > 0.0 {
                unexplained / energy
            } else {
                1.0
            };
            if !ambiguity_candidate && fraction > 0.1 {
                return no(UnsupportedModel);
            }
            segments.push(StepSegment {
                start_ms: (fit.anchor * 1000.0).round() as u32,
                ordinal: a as u32,
                bpm,
                evidence: StepEvidence {
                    sure: beat.sure,
                    catch: beat.catch,
                    max_residual_ms: residual * 1000.0,
                    supported_fraction: coverage,
                    unexplained_fraction: fraction,
                },
            });
            previous = Some((fit.anchor, fit.period, a));
        }
        Ok(Ok(segments))
    }
}
fn phase_distance(a: f64, b: f64, p: f64) -> f64 {
    let x = (a - b).rem_euclid(p);
    x.min(p - x)
}
struct Budget(usize);
impl Budget {
    fn tick(&mut self) -> Result<(), ResourceLimit> {
        self.0 += 1;
        if self.0 > MAX_CANDIDATES {
            Err(ResourceLimit)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy, Debug)]
struct Fit {
    anchor: f64,
    period: f64,
    residual: f64,
    sse: f64,
    coverage: f64,
    unexplained: f64,
}
fn linear_fit(
    peaks: &[f64],
    a: usize,
    b: usize,
    budget: &mut Budget,
) -> Result<Fit, ResourceLimit> {
    budget.tick()?;
    let n = (b - a + 1) as f64;
    let mean = (n - 1.0) / 2.0;
    let y = peaks[a..=b].iter().sum::<f64>() / n;
    let period = peaks[a..=b]
        .iter()
        .enumerate()
        .map(|(i, t)| (i as f64 - mean) * (t - y))
        .sum::<f64>()
        / (n * (n * n - 1.0) / 12.0);
    let anchor = y - mean * period;
    let mut residual = 0.0f64;
    let mut sse = 0.0;
    for (i, t) in peaks[a..=b].iter().enumerate() {
        let e = (t - anchor - i as f64 * period).abs();
        residual = residual.max(e);
        sse += e * e;
    }
    Ok(Fit {
        anchor,
        period,
        residual,
        sse,
        coverage: 1.0,
        unexplained: 0.0,
    })
}
fn pulse_fit(
    peaks: &[f64],
    anchor: f64,
    period: f64,
    end: f64,
    envelope: &[f32],
    first_time: f64,
) -> Fit {
    let anchor = anchor - ((anchor - peaks[0]) / period).floor() * period;
    let count = ((end - anchor) / period).ceil() as usize;
    let mut matched = 0;
    let mut used = Vec::with_capacity(MAX_PEAKS);
    let mut residual = 0.0f64;
    for i in 0..count {
        let t = anchor + i as f64 * period;
        if let Some(&peak) = peaks.iter().find(|&&p| (p - t).abs() <= 0.03) {
            matched += 1;
            used.push(peak);
            residual = residual.max((peak - t).abs());
        }
    }
    let energy = envelope.iter().map(|&x| f64::from(x.max(0.0))).sum::<f64>();
    let unexplained = envelope
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            let t = first_time + *i as f64 * 0.01;
            used.iter().all(|p| (p - t).abs() > 0.030000001)
        })
        .map(|(_, x)| f64::from(x.max(0.0)))
        .sum::<f64>();
    Fit {
        anchor,
        period,
        residual,
        sse: 0.0,
        coverage: matched as f64 / count.max(1) as f64,
        unexplained: if energy > 0.0 {
            unexplained / energy
        } else {
            1.0
        },
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dense_search_exhausts_budget_without_partial_map() {
        let mut b = StepTempoBuilder::new(8000);
        b.frames = 180 * 8000;
        // Adversarial bounded envelope: irregular dense peaks make the slice
        // search hit its budget, rather than allocating a larger search space.
        b.onsets.resize(MAX_ONSETS, 0.0);
        let mut i = 1;
        let mut n = 0;
        while i + 1 < MAX_ONSETS {
            b.onsets[i] = 1.0;
            n += 1;
            i += if n % 2 == 0 { 8 } else { 9 };
        }
        assert_eq!(b.onsets.capacity(), MAX_ONSETS);
        assert_eq!(b.finish(), Err(ResourceLimit));
        assert!(StepTempoBuilder::heap_bound_bytes() < 8 * 1024 * 1024);
    }
    #[test]
    fn candidate_cap_is_exact() {
        let mut b = Budget(0);
        for _ in 0..MAX_CANDIDATES {
            b.tick().unwrap();
        }
        assert_eq!(b.tick(), Err(ResourceLimit));
    }
}
