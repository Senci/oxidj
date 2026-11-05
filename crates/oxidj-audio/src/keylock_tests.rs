//! M8/D3 engine tests: key lock (Signalsmith behind [`TimeStretch`]) and
//! SYNC, on synthetic constant-BPM click tracks and sines rendered through
//! the exact callback code ([`Renderer::render`]).

use super::*;

const RATE: u32 = 48_000;

/// Which channel(s) a synthetic track sounds on.
#[derive(Clone, Copy)]
enum Side {
    Both,
    Left,
    Right,
}

/// Constant-BPM click track: a 2 ms decaying 2 kHz burst of peak `amp`
/// starting at `anchor_ms + k × 60000/bpm`.
fn clicks(rate: u32, secs: f64, bpm: f64, anchor_ms: f64, amp: f32, side: Side) -> Arc<TrackPcm> {
    let frames = (secs * f64::from(rate)) as usize;
    let mut samples = vec![0.0f32; frames * 2];
    let beat = 60.0 / bpm * f64::from(rate);
    let burst = (f64::from(rate) * 0.002) as usize;
    let mut k = 0.0;
    loop {
        let start = (anchor_ms / 1000.0 * f64::from(rate) + k * beat).ceil() as usize;
        if start >= frames {
            break;
        }
        for i in 0..burst.min(frames - start) {
            let t = i as f64 / f64::from(rate);
            let v = ((2.0 * std::f64::consts::PI * 2000.0 * t).sin() * (-t * 1500.0).exp()) as f32
                * amp;
            let f = (start + i) * 2;
            match side {
                Side::Both => (samples[f], samples[f + 1]) = (v, v),
                Side::Left => samples[f] = v,
                Side::Right => samples[f + 1] = v,
            }
        }
        k += 1.0;
    }
    Arc::new(TrackPcm::new(rate, samples).unwrap())
}

fn sine(rate: u32, secs: f64, hz: f64) -> Arc<TrackPcm> {
    let frames = (secs * f64::from(rate)) as usize;
    let samples = (0..frames)
        .flat_map(|i| {
            let v =
                (2.0 * std::f64::consts::PI * hz * i as f64 / f64::from(rate)).sin() as f32 * 0.5;
            [v, v]
        })
        .collect();
    Arc::new(TrackPcm::new(rate, samples).unwrap())
}

fn render(r: &mut Renderer, frames: usize) -> Vec<f32> {
    let mut out = vec![0.0; frames * 2];
    // Device-sized blocks, as the callback sees them.
    for block in out.chunks_mut(512 * 2) {
        r.render(block, 2);
    }
    out
}

/// Channel `ch` of interleaved stereo.
fn channel(out: &[f32], ch: usize) -> Vec<f32> {
    out.iter().skip(ch).step_by(2).copied().collect()
}

/// Onset times (ms at `RATE`) of bursts: first sample above 30 % of the
/// signal's peak after at least 100 ms below it.
fn onsets(x: &[f32]) -> Vec<f64> {
    let peak = x.iter().fold(0.0f32, |a, v| a.max(v.abs()));
    let threshold = peak * 0.3;
    let gap = (RATE / 10) as usize;
    let mut quiet = gap;
    let mut times = vec![];
    for (i, v) in x.iter().enumerate() {
        if v.abs() >= threshold {
            if quiet >= gap {
                // Linear interpolation of the crossing between i-1 and i.
                let prev = x.get(i.wrapping_sub(1)).map_or(0.0, |p| p.abs());
                let frac = f64::from((threshold - prev) / (v.abs() - prev).max(1e-9));
                times.push((i as f64 - 1.0 + frac.clamp(0.0, 1.0)) * 1000.0 / f64::from(RATE));
            }
            quiet = 0;
        } else {
            quiet += 1;
        }
    }
    times
}

/// Dominant frequency (Hz) by counting rising zero crossings in the middle.
fn frequency(x: &[f32]) -> f64 {
    let x = &x[x.len() / 4..x.len() * 3 / 4];
    let crossings: Vec<f64> = x
        .windows(2)
        .enumerate()
        .filter(|(_, w)| w[0] <= 0.0 && w[1] > 0.0)
        .map(|(i, w)| i as f64 + f64::from(-w[0] / (w[1] - w[0])))
        .collect();
    let (first, last) = (crossings[0], crossings[crossings.len() - 1]);
    (crossings.len() - 1) as f64 / ((last - first) / f64::from(RATE))
}

fn mean_interval(times: &[f64]) -> f64 {
    (times[times.len() - 1] - times[0]) / (times.len() - 1) as f64
}

// ------------------------------------------------------- stretch engine

/// Ratio and length invariants of the bare [`TimeStretch`] engine: every
/// call consumes all input and fills all output (length), the click period
/// scales by exactly 1/tempo (ratio), and a click track peaking at 0.9 never
/// reaches full scale (no clip) at ±8 % and ±16 %.
#[test]
fn stretch_engine_ratio_length_and_no_clip_on_click_track() {
    let track = clicks(RATE, 20.0, 120.0, 10.0, 0.9, Side::Both);
    let input: Vec<f32> = track.samples.clone();
    for tempo in [0.84, 0.92, 1.0, 1.08, 1.16] {
        let mut s = Signalsmith::new(RATE);
        let (block, mut consumed, mut carry) = (512usize, 0usize, 0.0f64);
        let mut out = vec![];
        let mut buf = vec![0.0f32; block * 2];
        while consumed + 2 * block < track.frames() as usize {
            let want = block as f64 * tempo + carry;
            let n = want as usize;
            carry = want - n as f64;
            s.process(&input[consumed * 2..(consumed + n) * 2], &mut buf);
            out.extend_from_slice(&buf);
            consumed += n;
        }
        // Length: output frames = blocks × 512, input consumed = out × tempo.
        let out_frames = out.len() / 2;
        assert!(
            (consumed as f64 - out_frames as f64 * tempo).abs() < 1.0,
            "tempo {tempo}: consumed {consumed} for {out_frames}"
        );
        let peak = out.iter().fold(0.0f32, |a, v| a.max(v.abs()));
        assert!(peak > 0.3 && peak < 0.99, "tempo {tempo}: peak {peak}");
        let times = onsets(&channel(&out, 0));
        let interval = mean_interval(&times[2..]);
        assert!(
            (interval - 500.0 / tempo).abs() < 0.5,
            "tempo {tempo}: click period {interval} ms, want {}",
            500.0 / tempo
        );
    }
}

/// The engine's latency is what the deck compensates; it must be stable.
#[test]
fn stretch_engine_reports_latency_and_preroll() {
    let s = Signalsmith::new(RATE);
    let (input, output) = s.latency();
    assert!(input > 0 && output > 0);
    assert!(s.preroll_frames() >= input * 2);
}

// ------------------------------------------------------------- key lock

fn deck_with(track: Arc<TrackPcm>, deck: usize) -> (Engine, Renderer) {
    let (mut e, r) = Engine::manual(RATE);
    e.send(Command::Load { deck, id: 1, track });
    (e, r)
}

/// Key lock keeps a 1 kHz tone at 1 kHz at ±8/±16 % (44.1 kHz track on a
/// 48 kHz device); without key lock the tone follows the tempo (varispeed).
#[test]
fn key_lock_keeps_pitch_varispeed_does_not() {
    for speed in [0.84, 0.92, 1.08, 1.16] {
        for keylock in [true, false] {
            let (mut e, mut r) = deck_with(sine(44_100, 8.0, 1000.0), 0);
            e.send(Command::Speed { deck: 0, speed });
            e.send(Command::KeyLock {
                deck: 0,
                on: keylock,
            });
            e.send(Command::TogglePlay { deck: 0 });
            let out = render(&mut r, RATE as usize * 4);
            let hz = frequency(&channel(&out, 0));
            let want = if keylock { 1000.0 } else { 1000.0 * speed };
            assert!(
                (hz - want).abs() < 0.5,
                "speed {speed} key lock {keylock}: {hz} Hz"
            );
            assert_eq!(e.deck(0).keylock, keylock);
        }
    }
}

/// With key lock the deck still advances `speed × source rate` frames per
/// second and the clicks come every `beat / speed`.
#[test]
fn key_lock_advances_at_speed_and_clicks_follow_the_tempo() {
    let (mut e, mut r) = deck_with(clicks(44_100, 30.0, 120.0, 0.0, 0.9, Side::Both), 0);
    e.send(Command::Seek {
        deck: 0,
        frame: 44_100.0,
    });
    e.send(Command::Speed {
        deck: 0,
        speed: 1.08,
    });
    e.send(Command::KeyLock { deck: 0, on: true });
    e.send(Command::TogglePlay { deck: 0 });
    let out = render(&mut r, RATE as usize * 10);
    let advanced = e.deck(0).frame - 44_100.0;
    assert!(
        (advanced - 10.0 * 1.08 * 44_100.0).abs() < 2.0,
        "advanced {advanced}"
    );
    let times = onsets(&channel(&out, 0));
    assert!((mean_interval(&times) - 500.0 / 1.08).abs() < 0.5);
}

/// The published key-lock position is the audible one: after a seek the
/// first click is heard exactly when the position says it is (< 1 ms).
#[test]
fn key_lock_position_is_latency_compensated() {
    for speed in [0.92, 1.0, 1.16] {
        let (mut e, mut r) = deck_with(clicks(44_100, 30.0, 120.0, 0.0, 0.9, Side::Both), 0);
        // 300 ms before the click at 10.0 s.
        e.send(Command::Seek {
            deck: 0,
            frame: 9.7 * 44_100.0,
        });
        e.send(Command::Speed { deck: 0, speed });
        e.send(Command::KeyLock { deck: 0, on: true });
        e.send(Command::TogglePlay { deck: 0 });
        let out = render(&mut r, RATE as usize * 2);
        let first = onsets(&channel(&out, 0))[0];
        let want = 300.0 / speed;
        assert!(
            (first - want).abs() < 1.0,
            "speed {speed}: click at {first} ms, want {want}"
        );
    }
}

/// Toggling key lock while playing keeps the position continuous.
#[test]
fn key_lock_toggle_keeps_position() {
    let (mut e, mut r) = deck_with(clicks(44_100, 30.0, 120.0, 0.0, 0.9, Side::Both), 0);
    e.send(Command::TogglePlay { deck: 0 });
    render(&mut r, RATE as usize);
    let before = e.deck(0).frame;
    e.send(Command::KeyLock { deck: 0, on: true });
    render(&mut r, 512);
    let after = e.deck(0).frame;
    assert!((after - before - 512.0 * 44_100.0 / 48_000.0).abs() < 2.0);
    e.send(Command::KeyLock { deck: 0, on: false });
    render(&mut r, 512);
    assert!((e.deck(0).frame - after - 512.0 * 44_100.0 / 48_000.0).abs() < 2.0);
}

// ----------------------------------------------------------------- sync

/// Deck A (left channel) 120 BPM at 44.1 kHz, deck B (right) 126 BPM at
/// 48 kHz with different first beats; SYNC B to A, then measure the clicks
/// actually rendered on each channel.
fn synced(a_speed: f64, b_keylock: bool, a_keylock: bool) -> (Vec<f64>, Vec<f64>, Engine) {
    let (mut e, mut r) = Engine::manual(RATE);
    let (a_rate, b_rate) = (44_100.0, 48_000.0);
    let (a_bpm, b_bpm, a_anchor, b_anchor) = (120.0, 126.0, 123.0, 37.0);
    e.send(Command::Load {
        deck: 0,
        id: 1,
        track: clicks(44_100, 60.0, a_bpm, a_anchor, 0.9, Side::Left),
    });
    e.send(Command::Load {
        deck: 1,
        id: 2,
        track: clicks(48_000, 60.0, b_bpm, b_anchor, 0.9, Side::Right),
    });
    e.send(Command::Speed {
        deck: 0,
        speed: a_speed,
    });
    e.send(Command::KeyLock {
        deck: 0,
        on: a_keylock,
    });
    e.send(Command::KeyLock {
        deck: 1,
        on: b_keylock,
    });
    e.send(Command::Seek {
        deck: 1,
        frame: 5.0 * b_rate,
    });
    e.send(Command::TogglePlay { deck: 0 });
    e.send(Command::TogglePlay { deck: 1 });
    render(&mut r, RATE as usize * 2);
    let speed = a_bpm * a_speed / b_bpm;
    e.send(Command::Sync {
        deck: 1,
        master: 0,
        speed,
        beat: 60.0 / b_bpm * b_rate,
        anchor: b_anchor / 1000.0 * b_rate,
        master_beat: 60.0 / a_bpm * a_rate,
        master_anchor: a_anchor / 1000.0 * a_rate,
    });
    let out = render(&mut r, RATE as usize * 8);
    let (a, b) = (onsets(&channel(&out, 0)), onsets(&channel(&out, 1)));
    // Skip the first beat after the jump.
    (a[1..].to_vec(), b[1..].to_vec(), e)
}

/// Every B click lands within 1 ms of an A click, and the tempo ratio is
/// matched (same click period), with and without key lock.
#[test]
fn sync_matches_tempo_and_phase_within_one_ms() {
    for (a_speed, b_keylock, a_keylock) in [
        (1.0, false, false),
        (1.03, false, false),
        (0.95, true, false),
        (1.03, true, true),
    ] {
        let (a, b, e) = synced(a_speed, b_keylock, a_keylock);
        let label = format!("A speed {a_speed}, key lock A {a_keylock} B {b_keylock}");
        let period = 500.0 / a_speed;
        assert!(
            (mean_interval(&a) - period).abs() < 0.2,
            "{label}: A period"
        );
        assert!(
            (mean_interval(&b) - period).abs() < 0.2,
            "{label}: B period"
        );
        let mut worst = 0.0f64;
        for t in &b {
            let nearest = a
                .iter()
                .map(|x| (x - t).abs())
                .fold(f64::INFINITY, f64::min);
            worst = worst.max(nearest);
        }
        assert!(b.len() >= 12, "{label}: {} clicks", b.len());
        assert!(worst < 1.0, "{label}: worst phase error {worst:.3} ms");
        // The published positions agree with what is heard.
        let (sa, sb) = (e.deck(0).frame / 44_100.0, e.deck(1).frame / 48_000.0);
        let pa = ((sa * 1000.0 - 123.0) / 500.0).rem_euclid(1.0);
        let pb = ((sb * 1000.0 - 37.0) / (60_000.0 / 126.0)).rem_euclid(1.0);
        let err = ((pa - pb + 0.5).rem_euclid(1.0) - 0.5).abs() * period;
        assert!(err < 1.0, "{label}: snapshot phase error {err:.3} ms");
    }
}

/// SYNC against an empty master or itself changes nothing but the count.
#[test]
fn sync_without_a_master_track_is_ignored() {
    let (mut e, mut r) = deck_with(clicks(48_000, 10.0, 120.0, 0.0, 0.9, Side::Both), 1);
    e.send(Command::Seek {
        deck: 1,
        frame: 1234.0,
    });
    for master in [0, 1, 7] {
        e.send(Command::Sync {
            deck: 1,
            master,
            speed: 1.5,
            beat: 24_000.0,
            anchor: 0.0,
            master_beat: 24_000.0,
            master_anchor: 100.0,
        });
    }
    render(&mut r, 8);
    assert_eq!(e.deck(1).frame, 1234.0);
    assert!(!e.pending(1));
    e.send(Command::TogglePlay { deck: 1 });
    render(&mut r, 100);
    // Speed unchanged (1.0): 100 frames at equal rates.
    assert_eq!(e.deck(1).frame, 1334.0);
}
