//! D4 mixer DSP properties and engine integration (bypass, meters).

use super::*;
use crate::{CHUNK, Command, Engine, METERS, TrackPcm};
use std::sync::Arc;

const RATE: u32 = 48_000;

fn sine(hz: f64, secs: f64, amp: f32) -> Vec<f32> {
    let n = (secs * f64::from(RATE)) as usize;
    (0..n)
        .flat_map(|i| {
            let v = (2.0 * PI * hz * i as f64 / f64::from(RATE)).sin() as f32 * amp;
            [v, v]
        })
        .collect()
}

/// Run `buf` through one strip in callback-sized chunks.
fn run(strip: &mut Strip, buf: &mut [f32], p: &ChannelParams, out: f32) -> f32 {
    let mut peak = 0.0f32;
    for chunk in buf.chunks_mut(CHUNK * 2) {
        peak = peak.max(strip.process(chunk, p, out, f64::from(RATE), None));
    }
    peak
}

fn rms(v: &[f32]) -> f64 {
    (v.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / v.len().max(1) as f64).sqrt()
}

/// Steady-state gain (dB) of `p` on a sine at `hz` (first 0.3 s skipped).
fn gain_db(p: &ChannelParams, hz: f64) -> f64 {
    let input = sine(hz, 1.0, 0.5);
    let mut out = input.clone();
    let mut strip = Strip::new(f64::from(RATE));
    run(&mut strip, &mut out, p, 1.0);
    let skip = (0.3 * f64::from(RATE)) as usize * 2;
    20.0 * (rms(&out[skip..]) / rms(&input[skip..])).log10()
}

fn with_eq(db: [f32; 3]) -> ChannelParams {
    ChannelParams {
        eq: db.map(eq_gain),
        cue: false,
        ..ChannelParams::default()
    }
}

#[test]
fn eq_at_0_db_is_flat_unity_magnitude() {
    assert_eq!(db_to_gain(0.0), 1.0);
    assert_eq!(eq_gain(0.0), 1.0);
    assert_eq!(trim_gain(0.0), 1.0);
    let p = ChannelParams::default();
    for hz in [
        30.0, 100.0, 250.0, 790.0, 2_500.0, 6_000.0, 12_000.0, 18_000.0,
    ] {
        let db = gain_db(&p, hz);
        assert!(db.abs() < 0.05, "{hz} Hz: {db:+.3} dB at 0 dB EQ");
    }
}

#[test]
fn each_kill_removes_its_band_and_keeps_the_others() {
    let kill = EQ_KILL_DB;
    // Low kill: 40 Hz gone, mid and high untouched.
    let p = with_eq([kill, 0.0, 0.0]);
    assert!(
        gain_db(&p, 40.0) <= -48.0,
        "low kill {:.1}",
        gain_db(&p, 40.0)
    );
    assert!(gain_db(&p, 8_000.0).abs() < 0.1);
    // Mid kill: the geometric band centre is gone, the edges stay.
    let p = with_eq([0.0, kill, 0.0]);
    assert!(
        gain_db(&p, 790.0) <= -30.0,
        "mid kill {:.1}",
        gain_db(&p, 790.0)
    );
    assert!(gain_db(&p, 30.0).abs() < 0.2);
    assert!(gain_db(&p, 15_000.0).abs() < 0.2);
    // High kill: 14 kHz gone, lows untouched.
    let p = with_eq([0.0, 0.0, kill]);
    assert!(
        gain_db(&p, 14_000.0) <= -48.0,
        "high kill {:.1}",
        gain_db(&p, 14_000.0)
    );
    assert!(gain_db(&p, 60.0).abs() < 0.1);
    // All three killed: silence (isolator).
    let p = with_eq([kill; 3]);
    for hz in [40.0, 790.0, 14_000.0] {
        assert!(gain_db(&p, hz) <= -60.0, "{hz}");
    }
    println!(
        "kill depth: low@40Hz {:.1} dB, mid@790Hz {:.1} dB, high@14kHz {:.1} dB",
        gain_db(&with_eq([kill, 0.0, 0.0]), 40.0),
        gain_db(&with_eq([0.0, kill, 0.0]), 790.0),
        gain_db(&with_eq([0.0, 0.0, kill]), 14_000.0)
    );
    // Knob values below the stop are kill, never negative gain.
    assert_eq!(eq_gain(-90.0), 0.0);
    assert_eq!(eq_gain(f32::NEG_INFINITY), 0.0);
    assert_eq!(eq_gain(f32::NAN), 0.0);
    // Boost tops out at +6 dB.
    let db = gain_db(&with_eq([12.0, 0.0, 0.0]), 40.0);
    assert!((db - 6.0).abs() < 0.2, "low +6 dB: {db:.2}");
    let db = gain_db(&with_eq([-12.0, 0.0, 0.0]), 40.0);
    assert!((db + 12.0).abs() < 0.3, "low -12 dB: {db:.2}");
}

#[test]
fn trim_fader_and_filter_sweep() {
    let p = ChannelParams {
        trim: trim_gain(6.0),
        ..ChannelParams::default()
    };
    assert!((gain_db(&p, 1_000.0) - 6.0).abs() < 0.05);
    assert_eq!(trim_gain(40.0), db_to_gain(TRIM_DB));
    // Low-pass side closes the highs, keeps the lows.
    let lp = ChannelParams {
        filter: -1.0,
        ..ChannelParams::default()
    };
    assert!(gain_db(&lp, 5_000.0) <= -40.0);
    assert!(gain_db(&lp, 40.0).abs() < 3.0);
    // High-pass side closes the lows, keeps the highs.
    let hp = ChannelParams {
        filter: 1.0,
        ..ChannelParams::default()
    };
    assert!(gain_db(&hp, 100.0) <= -40.0);
    assert!(gain_db(&hp, 15_000.0).abs() < 3.0);
    // Half-way is in between (monotonic sweep).
    let half = ChannelParams {
        filter: -0.5,
        ..ChannelParams::default()
    };
    let (open, mid, closed) = (
        gain_db(&ChannelParams::default(), 3_000.0),
        gain_db(&half, 3_000.0),
        gain_db(&lp, 3_000.0),
    );
    assert!(open > mid && mid > closed, "{open} {mid} {closed}");
    // The dead zone is exactly "off".
    let input = sine(300.0, 0.5, 0.5);
    let (mut a, mut b) = (input.clone(), input);
    run(
        &mut Strip::new(48_000.0),
        &mut a,
        &ChannelParams::default(),
        1.0,
    );
    let near = ChannelParams {
        filter: FILTER_DEADZONE * 0.5,
        ..ChannelParams::default()
    };
    run(&mut Strip::new(48_000.0), &mut b, &near, 1.0);
    assert_eq!(a, b);
    // Fader / crossfader gain scales the output after the meter point.
    let input = sine(1_000.0, 0.5, 0.5);
    let (mut a, mut b) = (input.clone(), input);
    let mut s = Strip::new(48_000.0);
    s.gains[4] = 0.25;
    let peak_b = run(&mut s, &mut b, &ChannelParams::default(), 0.25);
    let peak_a = run(
        &mut Strip::new(48_000.0),
        &mut a,
        &ChannelParams::default(),
        1.0,
    );
    assert!((peak_a - peak_b).abs() < 1e-6, "meter is pre-fader");
    let skip = 4_800;
    assert!((rms(&b[skip..]) / rms(&a[skip..]) - 0.25).abs() < 1e-4);
}

/// Deterministic full-scale noise.
fn noise(frames: usize, seed: u32) -> Vec<f32> {
    let mut x = seed;
    (0..frames * 2)
        .map(|_| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (x >> 8) as f32 / (1u32 << 23) as f32 - 1.0
        })
        .collect()
}

fn subnormal(v: f32) -> bool {
    v != 0.0 && v.abs() < f32::MIN_POSITIVE
}

#[test]
fn no_nan_or_denormals_under_extreme_settings_silence_and_bad_input() {
    let mut strip = Strip::new(f64::from(RATE));
    let extreme = |i: usize| ChannelParams {
        trim: trim_gain(TRIM_DB),
        eq: [eq_gain(EQ_MAX_DB); 3],
        // Sweep across both filter sides and the dead zone.
        filter: ((i as f32 * 0.37).sin() * 1.2).clamp(-1.0, 1.0),
        fader: 1.0,
        assign: Assign::Thru,
        cue: false,
    };
    let mut buf = noise(RATE as usize * 2, 7);
    for (i, chunk) in buf.chunks_mut(CHUNK * 2).enumerate() {
        strip.process(chunk, &extreme(i), 1.0, f64::from(RATE), None);
        assert!(chunk.iter().all(|s| s.is_finite()), "chunk {i}");
    }
    // 20 s of silence afterwards: decays to exact zero, never subnormal.
    let mut silent = vec![0.0f32; RATE as usize * 20 * 2];
    for chunk in silent.chunks_mut(CHUNK * 2) {
        strip.process(chunk, &extreme(3), 1.0, f64::from(RATE), None);
        assert!(chunk.iter().all(|s| s.is_finite() && !subnormal(*s)));
        for f in strip.biquads() {
            assert!(
                f.z.iter()
                    .flatten()
                    .all(|z| *z == 0.0 || z.abs() >= DENORMAL)
            );
        }
    }
    assert!(
        silent[silent.len() - CHUNK * 2..].iter().all(|s| *s == 0.0),
        "silence decays to zero"
    );
    // A NaN / infinite sample is dropped with its chunk; the strip recovers.
    let mut bad = sine(1_000.0, 0.1, 0.5);
    bad[10] = f32::NAN;
    bad[11] = f32::INFINITY;
    let p = ChannelParams::default();
    run(&mut strip, &mut bad, &p, 1.0);
    assert!(bad.iter().all(|s| s.is_finite()));
    let mut after = sine(1_000.0, 0.5, 0.5);
    let peak = run(&mut strip, &mut after, &p, 1.0);
    assert!(after.iter().all(|s| s.is_finite()));
    assert!((peak - 0.5).abs() < 0.05, "recovered: peak {peak}");
    // Non-finite controls are sanitised.
    let nan = ChannelParams {
        trim: f32::NAN,
        eq: [f32::NAN, f32::INFINITY, -1.0],
        filter: f32::NAN,
        fader: f32::NAN,
        assign: Assign::A,
        cue: false,
    };
    let mut buf = sine(1_000.0, 0.1, 0.5);
    run(&mut strip, &mut buf, &nan, f32::NAN);
    assert!(buf.iter().all(|s| s.is_finite()));
}

#[test]
fn crossfader_curve_invariants() {
    for curve in [Curve::Smooth, Curve::Sharp, Curve::Cut] {
        assert_eq!(
            crossfader_gains(0.5, curve),
            [1.0, 1.0],
            "{curve:?}: centre = both unity"
        );
        assert_eq!(
            crossfader_gains(0.0, curve),
            [1.0, 0.0],
            "{curve:?}: full A"
        );
        assert_eq!(
            crossfader_gains(1.0, curve),
            [0.0, 1.0],
            "{curve:?}: full B"
        );
        assert_eq!(crossfader_gains(-3.0, curve), [1.0, 0.0]);
        assert_eq!(crossfader_gains(7.0, curve), [0.0, 1.0]);
        assert_eq!(crossfader_gains(f32::NAN, curve), [1.0, 1.0]);
        let mut last = [1.0f32, 0.0];
        for i in 0..=2_000 {
            let x = i as f32 / 2_000.0;
            let [a, b] = crossfader_gains(x, curve);
            assert!((0.0..=1.0).contains(&a) && (0.0..=1.0).contains(&b));
            assert!(a <= last[0] && b >= last[1], "{curve:?} monotonic at {x}");
            assert_eq!(
                a,
                crossfader_gains(1.0 - x, curve)[1],
                "{curve:?} symmetric at {x}"
            );
            if x <= 0.5 {
                assert_eq!(a, 1.0, "{curve:?}: A full on its half");
            }
            if x >= 0.5 {
                assert_eq!(b, 1.0, "{curve:?}: B full on its half");
            }
            last = [a, b];
        }
    }
    // Sharper curves hold the far channel longer.
    for i in 0..=1_000 {
        let x = 0.5 + i as f32 / 2_000.0;
        let g = |c| crossfader_gains(x, c)[0];
        assert!(
            g(Curve::Smooth) <= g(Curve::Sharp) && g(Curve::Sharp) <= g(Curve::Cut),
            "{x}"
        );
    }
    assert!(crossfader_gains(0.75, Curve::Smooth)[0] < 0.75);
    assert_eq!(crossfader_gains(0.9, Curve::Sharp)[0], 1.0);
    assert!(crossfader_gains(0.95, Curve::Sharp)[0] < 1.0);
    assert_eq!(crossfader_gains(0.98, Curve::Cut)[0], 1.0);
    assert!(crossfader_gains(0.995, Curve::Cut)[0] < 1.0);
    // Assignment: Thru ignores the crossfader.
    let mut p = MixerParams {
        crossfader: Some(1.0),
        ..MixerParams::default()
    };
    assert_eq!([p.channel_out(0), p.channel_out(1)], [0.0, 1.0]);
    p.channels[0].assign = Assign::Thru;
    assert_eq!(p.channel_out(0), 1.0);
    p.channels[0].assign = Assign::B;
    assert_eq!(p.channel_out(0), 1.0);
    p.crossfader = None;
    p.channels[1].assign = Assign::A;
    assert_eq!(
        [p.channel_out(0), p.channel_out(1)],
        [1.0, 1.0],
        "hidden crossfader"
    );
    assert_eq!(MixerParams::default().channel_out(0), 1.0, "default centre");
}

// ------------------------------------------------------------- engine

fn track(hz: f64, secs: f64, amp: f32) -> Arc<TrackPcm> {
    Arc::new(TrackPcm::new(RATE, sine(hz, secs, amp)).unwrap())
}

/// Both decks loaded and playing (deck B key-locked at +4 %).
fn playing() -> (Engine, crate::Renderer) {
    let (mut engine, renderer) = Engine::manual(RATE);
    for (deck, hz) in [(0, 220.0), (1, 3_000.0)] {
        engine.send(Command::Load {
            deck,
            id: deck as u64 + 1,
            track: track(hz, 10.0, 0.4),
        });
        engine.send(Command::TogglePlay { deck });
    }
    engine.send(Command::KeyLock { deck: 1, on: true });
    engine.send(Command::Speed {
        deck: 1,
        speed: 1.04,
    });
    (engine, renderer)
}

fn block(r: &mut crate::Renderer, frames: usize) -> Vec<f32> {
    let mut out = vec![0.0; frames * 2];
    r.render(&mut out, 2);
    out
}

fn heavy() -> MixerParams {
    let mut p = MixerParams {
        crossfader: Some(0.8),
        curve: Curve::Sharp,
        master: 0.7,
        ..MixerParams::default()
    };
    p.channels[0] = ChannelParams {
        trim: trim_gain(-4.0),
        eq: [eq_gain(EQ_KILL_DB), eq_gain(3.0), eq_gain(-6.0)],
        filter: -0.4,
        fader: 0.6,
        assign: Assign::A,
        cue: false,
    };
    p.channels[1].filter = 0.5;
    p
}

#[test]
fn hidden_mixer_is_bit_identical_to_no_mixer() {
    let (ref_engine, mut reference) = playing();
    let (mut engine, mut mixed) = playing();
    // Mixer on with heavy settings: the output differs.
    engine.send(Command::Mixer(Some(heavy())));
    let (r, m) = (block(&mut reference, 9_000), block(&mut mixed, 9_000));
    assert!(
        r.iter().zip(&m).any(|(a, b)| a.to_bits() != b.to_bits()),
        "mixer in the path"
    );
    assert_eq!(
        engine.deck(0).frame,
        ref_engine.deck(0).frame,
        "positions unaffected"
    );
    // Hidden again: bit-identical, block after block, odd sizes too.
    engine.send(Command::Mixer(None));
    for frames in [512, 333, 1, 4_096] {
        let (r, m) = (block(&mut reference, frames), block(&mut mixed, frames));
        assert!(
            r.iter().zip(&m).all(|(a, b)| a.to_bits() == b.to_bits()),
            "bypass must be exact ({frames} frames)"
        );
    }
    engine.meters();
    block(&mut mixed, 512);
    assert_eq!(
        engine.meters(),
        [crate::Meter::default(); METERS],
        "no meters while hidden"
    );
}

#[test]
fn mixer_routes_faders_crossfader_and_meters() {
    let (mut engine, mut r) = playing();
    // Crossfader hard to B: deck A (220 Hz) is gone, deck B remains.
    engine.send(Command::Mixer(Some(MixerParams {
        crossfader: Some(1.0),
        ..MixerParams::default()
    })));
    block(&mut r, 4_800);
    engine.meters();
    let out = block(&mut r, 9_600);
    let left: Vec<f32> = out.iter().step_by(2).copied().collect();
    let low = goertzel(&left, 220.0);
    let high = goertzel(&left, 3_000.0);
    assert!(
        low < high * 1e-3,
        "A muted by the crossfader: {low} vs {high}"
    );
    let meters = engine.meters();
    assert!(
        (meters[0].peak - 0.4).abs() < 0.03,
        "A meter is pre-fader: {:?}",
        meters[0]
    );
    assert!(
        (meters[2].peak - 0.4).abs() < 0.05,
        "master = B only: {:?}",
        meters[2]
    );
    assert!(!meters.iter().any(|m| m.clip));
    assert_eq!(engine.meters()[0].peak, 0.0, "a read resets the peak");
    // Trim +12 dB on A at centre: channel and master clip; output clamped.
    let mut p = MixerParams::default();
    p.channels[0].trim = trim_gain(12.0);
    engine.send(Command::Mixer(Some(p)));
    let out = block(&mut r, 4_800);
    assert!(out.iter().all(|s| s.abs() <= 1.0));
    let meters = engine.meters();
    assert!(
        meters[0].clip && meters[2].clip && !meters[1].clip,
        "{meters:?}"
    );
    assert!(!engine.meters()[0].clip, "a read resets the clip flag");
    // Channel fader 0 and master 0.
    let mut p = MixerParams::default();
    p.channels[1].fader = 0.0;
    p.master = 0.0;
    engine.send(Command::Mixer(Some(p)));
    block(&mut r, 512);
    assert!(block(&mut r, 2_048).iter().all(|s| *s == 0.0));
}

/// Magnitude of `hz` in `x` (mono, at [`RATE`]).
fn goertzel(x: &[f32], hz: f64) -> f64 {
    let w = 2.0 * PI * hz / f64::from(RATE);
    let (mut s1, mut s2) = (0.0f64, 0.0f64);
    for v in x {
        let s = f64::from(*v) + 2.0 * w.cos() * s1 - s2;
        s2 = s1;
        s1 = s;
    }
    (s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2).sqrt()
}
