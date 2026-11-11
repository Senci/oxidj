//! DSP cost without a device: render blocks by hand and time each one.
//! Separates what the engine costs from what the OS scheduler adds
//! (compare with `qualify`, which measures inside a real callback).
//!
//! `cargo run --release -p oxidj-audio --example bench`

use cpal as _;
use oxidj_audio::{Command, DECLICK_MS, Engine, MixerParams, TrackPcm};
use rtrb as _;
use signalsmith_stretch as _;
use std::sync::Arc;
use std::time::Instant;

const RATE: u32 = 48_000;

fn run(block: usize, keylock: bool) {
    let (mut engine, mut renderer) = Engine::manual(RATE);
    renderer.set_declick_ms(DECLICK_MS);
    renderer.set_limiter_lookahead_ms(oxidj_audio::LIMITER_LOOKAHEAD_MS);
    // 44.1 kHz noise-like material: resampling on, every band busy.
    let mut seed = 0x9e37_79b9_u32;
    let samples: Vec<f32> = (0..44_100 * 40 * 2)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed as f32 / u32::MAX as f32 - 0.5) * 0.5
        })
        .collect();
    let track = Arc::new(TrackPcm::new(44_100, samples).expect("track"));
    engine.send(Command::Mixer(Some(MixerParams::default())));
    for deck in 0..2 {
        engine.send(Command::Load {
            deck,
            id: deck as u64,
            track: track.clone(),
        });
        engine.send(Command::KeyLock { deck, on: keylock });
        engine.send(Command::Speed {
            deck,
            speed: 1.08 - 0.12 * deck as f64,
        });
        engine.send(Command::TogglePlay { deck });
    }
    let mut out = vec![0.0_f32; block * 4];
    let blocks = RATE as usize * 30 / block;
    let mut times = Vec::with_capacity(blocks);
    for _ in 0..blocks {
        let t = Instant::now();
        renderer.render(&mut out, 4);
        times.push(t.elapsed().as_nanos() as u64);
    }
    times.sort_unstable();
    let period_ns = block as f64 * 1e9 / f64::from(RATE);
    let pct = |ns: u64| ns as f64 * 100.0 / period_ns;
    let mean = times.iter().sum::<u64>() as f64 / times.len() as f64;
    println!(
        "block={block:>4} keylock={keylock:<5} mean={:.1}% p50={:.1}% p99={:.1}% p99.9={:.1}% max={:.1}% ({:.3} ms)",
        mean * 100.0 / period_ns,
        pct(times[times.len() / 2]),
        pct(times[times.len() * 99 / 100]),
        pct(times[times.len() * 999 / 1000]),
        pct(times[times.len() - 1]),
        times[times.len() - 1] as f64 / 1e6,
    );
}

fn main() {
    println!("30 s of audio per row, two decks at 44.1 kHz into 48 kHz, mixer, 4 channels");
    for keylock in [false, true] {
        for block in [64, 128, 256, 512] {
            run(block, keylock);
        }
    }
}
