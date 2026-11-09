//! Synthetic four-channel cue smoke. Never runs without explicit endpoint selection.
//! OXIDJ_AUDIO_DEVICE='<exact CPAL name>' cargo run -p oxidj-audio --example cue_output
//! Use --list to enumerate endpoints without opening a stream.
use cpal::traits::{DeviceTrait, HostTrait};
use oxidj_audio::{Command, Engine, MixerParams, OutputKind, TrackPcm};
use rtrb as _;
use signalsmith_stretch as _;
use std::{sync::Arc, time::Duration};

fn main() {
    if std::env::args().any(|a| a == "--list") {
        for d in cpal::default_host().output_devices().expect("enumerate") {
            println!("{}", d.description().expect("description").name());
        }
        return;
    }
    let selected = std::env::var("OXIDJ_AUDIO_DEVICE")
        .expect("select a temporary four-channel output explicitly");
    let mut engine = Engine::start();
    assert!(engine.error().is_none(), "{:?}", engine.error());
    assert!(
        matches!(engine.output(), OutputKind::Device { ref name, channels: 4, .. } if *name == selected)
    );
    println!("selected {}", engine.output());
    let mut params = MixerParams {
        master: 1.0,
        cue_mix: 0.0,
        cue_volume: 0.5,
        ..Default::default()
    };
    params.channels[0].cue = true;
    params.channels[0].fader = 0.0;
    params.channels[1].cue = false;
    engine.send(Command::Mixer(Some(params)));
    // A: 440 Hz (headphones only), B: 880 Hz (main only). Distinct L/R.
    for (deck, hz, amp) in [(0, 440.0, 0.2), (1, 880.0, 0.1)] {
        let pcm = (0..48_000 * 6)
            .flat_map(|i| {
                let s = (i as f32 * hz * std::f32::consts::TAU / 48_000.0).sin() * amp;
                [s, s * 0.5]
            })
            .collect();
        engine.send(Command::Load {
            deck,
            id: deck as u64,
            track: Arc::new(TrackPcm::new(48_000, pcm).expect("tone")),
        });
        engine.send(Command::TogglePlay { deck });
    }
    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(100));
        engine.poll();
        assert!(engine.error().is_none(), "{:?}", engine.error());
    }
    let stats = engine.stats();
    assert!(stats.callbacks > 0);
    assert_eq!(stats.errors, 0);
    println!(
        "callbacks={} errors={} xruns={}",
        stats.callbacks, stats.errors, stats.xruns
    );
}
