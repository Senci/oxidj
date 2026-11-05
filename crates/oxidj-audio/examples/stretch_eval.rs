//! ADR-008 D3 stretch evaluation (`cargo run --release -p oxidj-audio --example stretch_eval`): Signalsmith Stretch (signalsmith-stretch
//! 0.1.3) vs a minimal in-house phase vocoder, on synthetic signals only.
//! Metrics: CPU per deck (stereo, 48 kHz, 512-frame blocks), pitch error and
//! spectral purity on a 1 kHz sine, transient smear on a click track, output
//! peak (clipping), callback allocations (glibc mallinfo2).
use cpal as _;
use rtrb as _;
use std::f64::consts::PI;
use std::time::Instant;

const SR: usize = 48_000;
const BLOCK: usize = 512;

#[repr(C)]
#[derive(Default)]
struct MallInfo2 {
    arena: usize,
    ordblks: usize,
    smblks: usize,
    hblks: usize,
    hblkhd: usize,
    usmblks: usize,
    fsmblks: usize,
    uordblks: usize,
    fordblks: usize,
    keepcost: usize,
}
unsafe extern "C" {
    fn mallinfo2() -> MallInfo2;
}
fn heap_in_use() -> usize {
    let m = unsafe { mallinfo2() };
    m.uordblks + m.hblkhd
}

// ---------- signals (interleaved stereo) ----------
fn sine(secs: f64, hz: f64, amp: f32) -> Vec<f32> {
    let n = (secs * SR as f64) as usize;
    (0..n)
        .flat_map(|i| {
            let s = (2.0 * PI * hz * i as f64 / SR as f64).sin() as f32 * amp;
            [s, s]
        })
        .collect()
}
/// 2 ms decaying 2 kHz bursts every 500 ms (120 BPM), peak `amp`.
fn clicks(secs: f64, amp: f32) -> Vec<f32> {
    let n = (secs * SR as f64) as usize;
    let period = SR / 2;
    (0..n)
        .flat_map(|i| {
            let t = i % period;
            let s = if t < SR / 500 {
                let tt = t as f64 / SR as f64;
                ((2.0 * PI * 2000.0 * tt).sin() * (-tt * 1500.0).exp()) as f32 * amp
            } else {
                0.0
            };
            [s, s]
        })
        .collect()
}
fn music_like(secs: f64) -> Vec<f32> {
    let a = sine(secs, 220.0, 0.25);
    let b = sine(secs, 1330.0, 0.15);
    let c = clicks(secs, 0.4);
    let mut seed = 1u32;
    a.iter()
        .zip(&b)
        .zip(&c)
        .map(|((x, y), z)| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            x + y + z + ((seed >> 9) as f32 / (1u32 << 23) as f32 - 0.5) * 0.05
        })
        .collect()
}

// ---------- FFT (radix-2, in place) ----------
fn fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = 2.0 * PI / len as f64 * if inverse { 1.0 } else { -1.0 };
        let (wr, wi) = (ang.cos(), ang.sin());
        for start in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let tr = re[b] * cr - im[b] * ci;
                let ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let nr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = nr;
            }
        }
        len <<= 1;
    }
}

// ---------- in-house phase vocoder (offline, per channel) ----------
/// Classic PV with identity phase locking: N=2048, synthesis hop N/4,
/// analysis hop = synthesis hop * tempo. Output length = input / tempo.
fn pv_channel(x: &[f64], tempo: f64) -> Vec<f64> {
    let n = 2048;
    let hs = n / 4;
    let ha = hs as f64 * tempo;
    let win: Vec<f64> = (0..n)
        .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f64 / n as f64).cos())
        .collect();
    let out_len = (x.len() as f64 / tempo) as usize;
    let mut out = vec![0.0; out_len + n];
    let mut prev_phase = vec![0.0; n / 2 + 1];
    let mut sum_phase = vec![0.0; n / 2 + 1];
    let (mut re, mut im) = (vec![0.0; n], vec![0.0; n]);
    let mut mag = vec![0.0; n / 2 + 1];
    let mut ph = vec![0.0; n / 2 + 1];
    let mut new_phase = vec![0.0; n / 2 + 1];
    let mut first = true;
    let mut frame = 0usize;
    loop {
        let ip = (frame as f64 * ha) as usize;
        if ip + n > x.len() || frame * hs + n > out.len() {
            break;
        }
        for i in 0..n {
            re[i] = x[ip + i] * win[i];
            im[i] = 0.0;
        }
        fft(&mut re, &mut im, false);
        for k in 0..=n / 2 {
            mag[k] = re[k].hypot(im[k]);
            ph[k] = im[k].atan2(re[k]);
        }
        if first {
            new_phase.copy_from_slice(&ph);
            first = false;
        } else {
            for k in 0..=n / 2 {
                let omega = 2.0 * PI * k as f64 / n as f64;
                let d = ph[k] - prev_phase[k] - omega * ha;
                let d = d - 2.0 * PI * (d / (2.0 * PI)).round();
                new_phase[k] = sum_phase[k] + (omega + d / ha) * hs as f64;
            }
            // identity phase locking: bins follow their region's peak
            let mut k = 0;
            while k <= n / 2 {
                let lo = k;
                let mut p = k;
                while p < n / 2 && mag[p + 1] >= mag[p] {
                    p += 1;
                }
                let mut hi = p;
                while hi < n / 2 && mag[hi + 1] < mag[hi] {
                    hi += 1;
                }
                for b in lo..=hi {
                    if b != p {
                        new_phase[b] = new_phase[p] + (ph[b] - ph[p]);
                    }
                }
                k = hi + 1;
            }
        }
        prev_phase.copy_from_slice(&ph);
        sum_phase.copy_from_slice(&new_phase);
        for k in 0..=n / 2 {
            re[k] = mag[k] * new_phase[k].cos();
            im[k] = mag[k] * new_phase[k].sin();
            if k > 0 && k < n / 2 {
                re[n - k] = re[k];
                im[n - k] = -im[k];
            }
        }
        fft(&mut re, &mut im, true);
        let o = frame * hs;
        // Hann^2 OLA at 75% overlap sums to 1.5
        for i in 0..n {
            out[o + i] += re[i] / n as f64 * win[i] / 1.5;
        }
        frame += 1;
    }
    out.truncate(out_len);
    out
}

fn pv(input: &[f32], tempo: f64) -> Vec<f32> {
    let l: Vec<f64> = input.iter().step_by(2).map(|&s| s as f64).collect();
    let r: Vec<f64> = input.iter().skip(1).step_by(2).map(|&s| s as f64).collect();
    let (ol, or) = (pv_channel(&l, tempo), pv_channel(&r, tempo));
    ol.iter()
        .zip(&or)
        .flat_map(|(a, b)| [*a as f32, *b as f32])
        .collect()
}

fn signalsmith(input: &[f32], tempo: f64) -> Vec<f32> {
    let mut s = signalsmith_stretch::Stretch::preset_default(2, SR as u32);
    let frames_in = input.len() / 2;
    let frames_out = (frames_in as f64 / tempo) as usize;
    let mut out = vec![0.0f32; frames_out * 2];
    let mut consumed = 0.0f64;
    let mut written = 0;
    while written < frames_out {
        let n = BLOCK.min(frames_out - written);
        let a = consumed as usize;
        consumed += n as f64 * tempo;
        let b = (consumed as usize).min(frames_in);
        s.process(
            &input[a * 2..b * 2],
            &mut out[written * 2..(written + n) * 2],
        );
        written += n;
    }
    out
}

// ---------- metrics ----------
fn mono(x: &[f32]) -> Vec<f64> {
    x.iter().step_by(2).map(|&s| s as f64).collect()
}
/// Dominant frequency (Hz) via 32768-point FFT of a steady section and
/// spurious level: energy outside +-30 Hz of the peak relative to inside (dB).
fn tone(x: &[f32]) -> (f64, f64) {
    let m = mono(x);
    let n = 32768;
    let start = m.len() / 2 - n / 2;
    let mut re: Vec<f64> = (0..n)
        .map(|i| m[start + i] * (0.5 - 0.5 * (2.0 * PI * i as f64 / n as f64).cos()))
        .collect();
    let mut im = vec![0.0; n];
    fft(&mut re, &mut im, false);
    let p: Vec<f64> = (0..n / 2).map(|k| re[k] * re[k] + im[k] * im[k]).collect();
    let k = (1..n / 2 - 1)
        .max_by(|&a, &b| p[a].total_cmp(&p[b]))
        .unwrap();
    let (a, b, c) = (p[k - 1].ln(), p[k].ln(), p[k + 1].ln());
    let delta = 0.5 * (a - c) / (a - 2.0 * b + c);
    let hz = (k as f64 + delta) * SR as f64 / n as f64;
    let w = (30.0 * n as f64 / SR as f64) as usize;
    let inside: f64 = p[k.saturating_sub(w)..(k + w).min(n / 2)].iter().sum();
    let total: f64 = p.iter().sum();
    (hz, 10.0 * ((total - inside).max(1e-30) / inside).log10())
}
/// Mean width (ms) above -20 dB of each click's peak, and pre-echo: energy in
/// the 20 ms before the -20 dB onset relative to the following 10 ms (dB).
fn smear(x: &[f32], period: usize) -> (f64, f64) {
    let m = mono(x);
    let mut widths = vec![];
    let mut pre = vec![];
    for c in 2..(m.len() / period).saturating_sub(2) {
        let seg = &m[c * period - period / 2..c * period + period / 2];
        let pk = seg.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let thr = pk * 0.1;
        let first = seg.iter().position(|v| v.abs() >= thr).unwrap();
        let last = seg.len() - 1 - seg.iter().rev().position(|v| v.abs() >= thr).unwrap();
        widths.push((last - first) as f64 * 1000.0 / SR as f64);
        let pre_n = SR / 50;
        let e_pre: f64 = seg[first.saturating_sub(pre_n)..first]
            .iter()
            .map(|v| v * v)
            .sum();
        let e_click: f64 = seg[first..(first + SR / 100).min(seg.len())]
            .iter()
            .map(|v| v * v)
            .sum();
        pre.push(10.0 * (e_pre.max(1e-30) / e_click).log10());
    }
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    (mean(&widths), mean(&pre))
}
fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0, |a, v| a.max(v.abs()))
}

fn cpu_signalsmith(tempo: f64, cheaper: bool) -> (f64, f64, isize, usize) {
    let secs = 60.0;
    let input = music_like(secs * tempo + 1.0);
    let mut s = if cheaper {
        signalsmith_stretch::Stretch::preset_cheaper(2, SR as u32)
    } else {
        signalsmith_stretch::Stretch::preset_default(2, SR as u32)
    };
    let mut out = vec![0.0f32; BLOCK * 2];
    let blocks = (secs * SR as f64) as usize / BLOCK;
    s.process(&input[..BLOCK * 2], &mut out);
    let mut consumed = BLOCK as f64;
    let mut worst: f64 = 0.0;
    let h0 = heap_in_use();
    let mut grew = 0usize;
    let t = Instant::now();
    for _ in 0..blocks {
        let a = consumed as usize;
        consumed += BLOCK as f64 * tempo;
        let b = consumed as usize;
        let tb = Instant::now();
        let before = heap_in_use();
        s.process(&input[a * 2..b * 2], &mut out);
        if heap_in_use() != before {
            grew += 1;
        }
        worst = worst.max(tb.elapsed().as_secs_f64());
    }
    let total = t.elapsed().as_secs_f64();
    (
        total / secs * 100.0,
        worst * 1000.0,
        heap_in_use() as isize - h0 as isize,
        grew,
    )
}

/// One key-locked deck through the real callback code.
fn engine_cpu(speed: f64) -> (f64, f64) {
    use oxidj_audio::{Command, Engine, TrackPcm};
    let secs = 60.0;
    // music_like is generated at 48 kHz; played as 44.1 kHz material it
    // also exercises the rate-conversion step.
    let pcm = TrackPcm::new(44_100, music_like(secs * speed + 2.0)).expect("pcm");
    let (mut engine, mut renderer) = Engine::manual(SR as u32);
    engine.send(Command::Load {
        deck: 0,
        id: 1,
        track: std::sync::Arc::new(pcm),
    });
    engine.send(Command::Speed { deck: 0, speed });
    engine.send(Command::KeyLock { deck: 0, on: true });
    engine.send(Command::TogglePlay { deck: 0 });
    let mut out = vec![0.0f32; BLOCK * 2];
    renderer.render(&mut out, 2);
    let blocks = (secs * SR as f64) as usize / BLOCK;
    let mut worst: f64 = 0.0;
    let t = Instant::now();
    for _ in 0..blocks {
        let tb = Instant::now();
        renderer.render(&mut out, 2);
        worst = worst.max(tb.elapsed().as_secs_f64());
    }
    (t.elapsed().as_secs_f64() / secs * 100.0, worst * 1000.0)
}

fn main() {
    let tempos = [0.84, 0.92, 1.08, 1.16];
    println!("# CPU per deck (stereo 48 kHz, 512-frame blocks, 60 s, release)");
    for &t in &[0.84, 1.0, 1.16] {
        for cheaper in [false, true] {
            let (pct, worst, heap, grew) = cpu_signalsmith(t, cheaper);
            println!(
                "signalsmith {} tempo {t:.2}: {pct:.2}% of one core, worst block {worst:.3} ms (budget 10.667), heap delta {heap} B, blocks with heap change {grew}",
                if cheaper { "cheaper" } else { "default" }
            );
        }
        let input = music_like(60.0 * t + 1.0);
        let tm = Instant::now();
        let _ = pv(&input, t);
        println!(
            "in-house PV tempo {t:.2}: {:.2}% of one core (offline, f64, naive FFT)",
            tm.elapsed().as_secs_f64() / 60.0 * 100.0
        );
    }
    println!("\n# 1 kHz sine, key lock (pitch must stay 1000 Hz)");
    let s = sine(8.0, 1000.0, 0.9);
    for &t in &tempos {
        let (a, b) = (signalsmith(&s, t), pv(&s, t));
        let ((fa, sa), (fb, sb)) = (tone(&a), tone(&b));
        println!(
            "tempo {:+.0}%: signalsmith {fa:.2} Hz ({:+.2} cents) spurious {sa:.1} dB peak {:.3} | PV {fb:.2} Hz ({:+.2} cents) spurious {sb:.1} dB peak {:.3}",
            (t - 1.0) * 100.0,
            1200.0 * (fa / 1000.0).log2(),
            peak(&a),
            1200.0 * (fb / 1000.0).log2(),
            peak(&b)
        );
    }
    println!("\n# click track 120 BPM, 2 ms bursts peak 0.9");
    let c = clicks(12.0, 0.9);
    let (w0, p0) = smear(&c, SR / 2);
    println!("input: width {w0:.2} ms, pre-echo {p0:.1} dB");
    for &t in &tempos {
        let per = ((SR / 2) as f64 / t) as usize;
        let (a, b) = (signalsmith(&c, t), pv(&c, t));
        let ((wa, pa), (wb, pb)) = (smear(&a, per), smear(&b, per));
        println!(
            "tempo {:+.0}%: signalsmith width {wa:.2} ms pre-echo {pa:.1} dB peak {:.3} | PV width {wb:.2} ms pre-echo {pb:.1} dB peak {:.3}",
            (t - 1.0) * 100.0,
            peak(&a),
            peak(&b)
        );
    }
    println!("\n# OxiDJ deck engine, key lock on (Renderer::render, 60 s, 512-frame blocks)");
    for &t in &[0.84, 1.0, 1.16] {
        let (pct, worst) = engine_cpu(t);
        println!(
            "one key-locked deck at speed {t:.2}: {pct:.2}% of one core, worst block {worst:.3} ms"
        );
    }
    let s = signalsmith_stretch::Stretch::preset_default(2, SR as u32);
    println!(
        "\nlatency signalsmith default: input {} + output {} frames",
        s.input_latency(),
        s.output_latency()
    );
}
