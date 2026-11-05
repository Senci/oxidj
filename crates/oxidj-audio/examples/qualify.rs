//! ADR-008 D2 qualification run: open the default output device, load a
//! synthetic tone on BOTH decks at gain 0 (the full two-deck mix runs, the
//! speakers stay silent) and print callback timing and xrun counts.
//!
//! `cargo run --release -p oxidj-audio --example qualify -- <seconds>`

use cpal as _;
use oxidj_audio::{Command, Engine, OutputKind, TrackPcm};
use rtrb as _;
use signalsmith_stretch as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() {
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(600);
    let mut engine = Engine::start();
    if let Some(error) = engine.error() {
        println!("NO DEVICE: {error}");
        std::process::exit(2);
    }
    let OutputKind::Device {
        name,
        rate,
        channels,
    } = engine.output()
    else {
        println!("NO DEVICE");
        std::process::exit(2);
    };
    println!("device: {name} · {rate} Hz · {channels} ch");
    // 44.1 kHz tone so a 48 kHz device exercises the fractional-step path.
    let samples: Vec<f32> = (0..44_100 * 60 * 2)
        .map(|i| ((i / 2) as f32 * 440.0 * std::f32::consts::TAU / 44_100.0).sin() * 0.1)
        .collect();
    let track = Arc::new(TrackPcm::new(44_100, samples).expect("tone"));
    for deck in 0..2 {
        engine.send(Command::Load {
            deck,
            id: deck as u64,
            track: track.clone(),
        });
        engine.send(Command::Gain { deck, gain: 0.0 });
        engine.send(Command::LoopIn { deck });
    }
    std::thread::sleep(Duration::from_millis(200));
    for deck in 0..2 {
        engine.send(Command::TogglePlay { deck });
    }
    // Loop the first ~30 s so a 10 min run never reaches the end.
    std::thread::sleep(Duration::from_secs(30));
    for deck in 0..2 {
        engine.send(Command::LoopOut { deck });
    }
    let start = Instant::now();
    let base = engine.stats();
    let mut last_print = Instant::now();
    while start.elapsed() < Duration::from_secs(seconds) {
        std::thread::sleep(Duration::from_millis(250));
        engine.poll();
        if engine.error().is_some() {
            println!("DEVICE LOST: {}", engine.error().unwrap_or_default());
            std::process::exit(3);
        }
        if last_print.elapsed() >= Duration::from_secs(60) {
            last_print = Instant::now();
            let s = engine.stats();
            println!(
                "t={:>4}s callbacks={} xruns={} errors={} max_interval={:.2}ms playing={}/{}",
                start.elapsed().as_secs(),
                s.callbacks - base.callbacks,
                s.xruns - base.xruns,
                s.errors - base.errors,
                s.max_interval_ns as f64 / 1e6,
                engine.deck(0).playing,
                engine.deck(1).playing
            );
        }
    }
    let s = engine.stats();
    let callbacks = s.callbacks - base.callbacks;
    let frames = s.frames - base.frames;
    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "RESULT seconds={elapsed:.1} device_rate={rate} callbacks={callbacks} frames={frames} \
         block_min={} block_max={} mean_period={:.3}ms max_interval={:.3}ms xruns={} errors={} \
         frames_per_second={:.1}",
        s.min_block,
        s.max_block,
        elapsed * 1000.0 / callbacks.max(1) as f64,
        s.max_interval_ns as f64 / 1e6,
        s.xruns - base.xruns,
        s.errors - base.errors,
        frames as f64 / elapsed
    );
}
