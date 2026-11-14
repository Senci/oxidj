//! ADR-008 D2 qualification run: open the default output device, load a
//! synthetic tone on BOTH decks at gain 0 (the full two-deck mix runs, the
//! speakers stay silent) and print callback timing and xrun counts.
//!
//! `cargo run --release -p oxidj-audio --example qualify -- <seconds>`
//! `... --example qualify -- --list` prints every output device.
//! The output is chosen with `OXIDJ_AUDIO_HOST`, `OXIDJ_AUDIO_DEVICE`,
//! `OXIDJ_AUDIO_RATE`, `OXIDJ_AUDIO_BUFFER`, `OXIDJ_AUDIO_CHANNELS`.

use cpal as _;
use oxidj_audio::{Command, Engine, MixerParams, OutputKind, TrackPcm};
use rtrb as _;
use signalsmith_stretch as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn main() {
    if std::env::args().any(|a| a == "--list") {
        for o in oxidj_audio::list_outputs() {
            println!(
                "{} | {} | {}{} | up to {} ch | {}-{} Hz | buffer {:?}",
                o.host,
                o.id,
                o.device,
                if o.default { " (default)" } else { "" },
                o.max_channels,
                o.rates.0,
                o.rates.1,
                o.buffer_frames
            );
        }
        return;
    }
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(600);
    // `--varispeed`: tempo without key lock (no time stretch in the path).
    let keylock = !std::env::args().any(|a| a == "--varispeed");
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
    println!(
        "device: {name} · {rate} Hz · {channels} ch · {:?}",
        engine.config()
    );
    // 44.1 kHz tone so a 48 kHz device exercises the resampling path.
    let samples: Vec<f32> = (0..44_100 * 60 * 2)
        .map(|i| ((i / 2) as f32 * 440.0 * std::f32::consts::TAU / 44_100.0).sin() * 0.1)
        .collect();
    let track = Arc::new(TrackPcm::new(44_100, samples).expect("tone"));
    // Worst-case load: both decks key-locked off-tempo through the full
    // mixer; master volume 0 keeps the speakers silent.
    engine.send(Command::Mixer(Some(MixerParams {
        master: 0.0,
        cue_volume: 0.0,
        ..MixerParams::default()
    })));
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
    engine.reset_stats();
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
                "t={:>4}s callbacks={} xruns={} errors={} max_interval={:.2}ms load={:.1}% playing={}/{}",
                start.elapsed().as_secs(),
                s.callbacks - base.callbacks,
                s.xruns - base.xruns,
                s.errors - base.errors,
                s.max_interval_ns as f64 / 1e6,
                f64::from(s.max_load_permille) / 10.0,
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
         frames_per_second={:.1} max_render={:.3}ms max_load={:.1}% mean_load={:.1}% \
         latency={:.2}ms",
        s.min_block,
        s.max_block,
        elapsed * 1000.0 / callbacks.max(1) as f64,
        s.max_interval_ns as f64 / 1e6,
        s.xruns - base.xruns,
        s.errors - base.errors,
        frames as f64 / elapsed,
        s.max_render_ns as f64 / 1e6,
        f64::from(s.max_load_permille) / 10.0,
        (s.render_ns - base.render_ns) as f64 / 1e7 / (frames as f64 / f64::from(rate)),
        s.latency_ns as f64 / 1e6
    );
}
