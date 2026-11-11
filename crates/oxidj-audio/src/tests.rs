//! Engine tests on the fake sink: blocks rendered by hand, exactly the code
//! the device callback runs.

use super::*;

const RATE: u32 = 48_000;

/// A track whose left sample at frame `i` is `i` (scaled) and right is `-i`,
/// so every output sample names the source frame it came from.
fn ramp(frames: usize, rate: u32) -> Arc<TrackPcm> {
    let samples = (0..frames)
        .flat_map(|i| {
            let v = i as f32 / 1_000_000.0;
            [v, -v]
        })
        .collect();
    Arc::new(TrackPcm::new(rate, samples).unwrap())
}

fn frame_of(sample: f32) -> f64 {
    f64::from(sample) * 1_000_000.0
}

fn setup(frames: usize) -> (Engine, Renderer) {
    let (mut engine, renderer) = Engine::manual(RATE);
    assert!(engine.send(Command::Load {
        deck: 0,
        id: 7,
        track: ramp(frames, RATE),
    }));
    (engine, renderer)
}

fn render(r: &mut Renderer, frames: usize) -> Vec<f32> {
    let mut out = vec![0.0; frames * 2];
    r.render(&mut out, 2);
    out
}

#[test]
fn track_pcm_rejects_bad_input() {
    assert!(TrackPcm::new(0, vec![0.0, 0.0]).is_none());
    assert!(TrackPcm::new(44_100, vec![0.0]).is_none());
    assert!(TrackPcm::new(44_100, vec![]).is_none());
}

#[test]
fn load_publishes_id_and_play_advances_sample_accurately() {
    let (mut e, mut r) = setup(10_000);
    render(&mut r, 16);
    let s = e.deck(0);
    assert_eq!((s.id, s.frame, s.playing), (Some(7), 0.0, false));
    // Paused: silence, position unchanged.
    assert!(render(&mut r, 64).iter().all(|&x| x == 0.0));
    e.send(Command::TogglePlay { deck: 0 });
    let out = render(&mut r, 100);
    // Equal rates: source samples copied unchanged, frame by frame.
    for i in 0..100 {
        assert_eq!(out[2 * i], i as f32 / 1_000_000.0);
        assert_eq!(out[2 * i + 1], -(i as f32) / 1_000_000.0);
    }
    assert_eq!(e.deck(0).frame, 100.0);
    assert!(e.deck(0).playing);
    render(&mut r, 37);
    assert_eq!(e.deck(0).frame, 137.0);
    e.send(Command::TogglePlay { deck: 0 });
    render(&mut r, 50);
    assert_eq!((e.deck(0).frame, e.deck(0).playing), (137.0, false));
}

#[test]
fn rate_conversion_44k1_track_on_48k_device_interpolates() {
    let (mut e, mut r) = Engine::manual(48_000);
    e.send(Command::Load {
        deck: 1,
        id: 1,
        track: ramp(100_000, 44_100),
    });
    e.send(Command::TogglePlay { deck: 1 });
    let out = render(&mut r, 48_000);
    // One device second = one source second, sample accurate.
    assert!((e.deck(1).frame - 44_100.0).abs() < 1e-6);
    // Output frame n reads source frame n·44100/48000 (linear on a ramp).
    for n in [1_usize, 1_000, 47_999] {
        let want = n as f64 * 44_100.0 / 48_000.0;
        assert!((frame_of(out[2 * n]) - want).abs() < 0.05, "frame {n}");
    }
}

#[test]
fn cue_cdj_semantics_set_at_stop_preview_while_held_and_return() {
    let (mut e, mut r) = setup(100_000);
    // Paused at 0 == cue: CUE down previews from the cue point while held.
    e.send(Command::Seek {
        deck: 0,
        frame: 500.0,
    });
    e.send(Command::CueDown { deck: 0 }); // paused off cue → sets cue
    render(&mut r, 8);
    assert_eq!((e.deck(0).cue, e.deck(0).playing), (500.0, false));
    e.send(Command::CueDown { deck: 0 }); // on cue → hold-preview
    render(&mut r, 200);
    let s = e.deck(0);
    assert!(s.playing && s.previewing);
    assert_eq!(s.frame, 700.0);
    e.send(Command::CueUp { deck: 0 }); // release → back to cue, paused
    render(&mut r, 8);
    let s = e.deck(0);
    assert_eq!((s.frame, s.playing, s.previewing), (500.0, false, false));
    // While playing: CUE stops and returns to the cue point.
    e.send(Command::TogglePlay { deck: 0 });
    render(&mut r, 300);
    assert_eq!(e.deck(0).frame, 800.0);
    e.send(Command::CueDown { deck: 0 });
    e.send(Command::CueUp { deck: 0 });
    render(&mut r, 8);
    assert_eq!((e.deck(0).frame, e.deck(0).playing), (500.0, false));
    // PLAY during a preview keeps playing after CUE is released.
    e.send(Command::CueDown { deck: 0 });
    render(&mut r, 10);
    e.send(Command::TogglePlay { deck: 0 });
    e.send(Command::CueUp { deck: 0 });
    render(&mut r, 10);
    let s = e.deck(0);
    assert!(s.playing && !s.previewing);
    assert_eq!(s.frame, 520.0);
}

#[test]
fn loop_wraps_sample_accurately_and_halve_double_exit() {
    let (mut e, mut r) = setup(100_000);
    e.send(Command::Seek {
        deck: 0,
        frame: 1_000.0,
    });
    e.send(Command::LoopIn { deck: 0 });
    e.send(Command::TogglePlay { deck: 0 });
    render(&mut r, 1_000);
    e.send(Command::LoopOut { deck: 0 }); // loop [1000, 2000)
    let out = render(&mut r, 1_500);
    assert_eq!(e.deck(0).loop_range, Some((1_000.0, 2_000.0)));
    // Frame 999 of this block is source 1999, frame 1000 wraps to 1000.
    assert_eq!(frame_of(out[2 * 999]).round(), 1_999.0);
    assert_eq!(frame_of(out[2 * 1000]).round(), 1_000.0);
    assert_eq!(e.deck(0).frame, 1_500.0);
    e.send(Command::LoopScale {
        deck: 0,
        factor: 0.5,
    }); // [1000,1500): wraps
    render(&mut r, 1);
    assert_eq!(e.deck(0).loop_range, Some((1_000.0, 1_500.0)));
    assert_eq!(e.deck(0).frame, 1_001.0);
    e.send(Command::LoopScale {
        deck: 0,
        factor: 2.0,
    });
    e.send(Command::LoopScale {
        deck: 0,
        factor: 2.0,
    });
    render(&mut r, 1);
    assert_eq!(e.deck(0).loop_range, Some((1_000.0, 3_000.0)));
    e.send(Command::LoopOut { deck: 0 }); // exit
    render(&mut r, 5_000);
    assert_eq!(e.deck(0).loop_range, None);
    assert_eq!(e.deck(0).frame, 6_002.0);
}

#[test]
fn too_short_or_backwards_loop_is_refused() {
    let (mut e, mut r) = setup(100_000);
    e.send(Command::Seek {
        deck: 0,
        frame: 1_000.0,
    });
    e.send(Command::LoopIn { deck: 0 });
    e.send(Command::Seek {
        deck: 0,
        frame: 1_100.0,
    }); // 2 ms < 10 ms
    e.send(Command::LoopOut { deck: 0 });
    e.send(Command::Seek {
        deck: 0,
        frame: 10.0,
    }); // before IN
    e.send(Command::LoopOut { deck: 0 });
    render(&mut r, 4);
    assert_eq!(e.deck(0).loop_range, None);
}

#[test]
fn seek_clamps_and_end_of_track_stops_exactly() {
    let (mut e, mut r) = setup(1_000);
    e.send(Command::Seek {
        deck: 0,
        frame: -50.0,
    });
    render(&mut r, 1);
    assert_eq!(e.deck(0).frame, 0.0);
    e.send(Command::Seek {
        deck: 0,
        frame: f64::NAN,
    });
    e.send(Command::Seek {
        deck: 0,
        frame: 900.0,
    });
    e.send(Command::TogglePlay { deck: 0 });
    let out = render(&mut r, 300);
    let s = e.deck(0);
    assert_eq!((s.frame, s.playing), (1_000.0, false));
    // 100 frames of audio, then silence.
    assert_eq!(frame_of(out[2 * 99]).round(), 999.0);
    assert!(out[200..].iter().all(|&x| x == 0.0));
    // PLAY at the end does nothing; a seek back allows it again.
    e.send(Command::TogglePlay { deck: 0 });
    render(&mut r, 1);
    assert!(!e.deck(0).playing);
    e.send(Command::Seek {
        deck: 0,
        frame: 5_000.0,
    });
    render(&mut r, 1);
    assert_eq!(e.deck(0).frame, 1_000.0);
}

#[test]
fn two_decks_mix_at_unity_and_gain_scales() {
    let (mut e, mut r) = Engine::manual(RATE);
    let dc = |v: f32| Arc::new(TrackPcm::new(RATE, vec![v; 2_000]).unwrap());
    e.send(Command::Load {
        deck: 0,
        id: 1,
        track: dc(0.25),
    });
    e.send(Command::Load {
        deck: 1,
        id: 2,
        track: dc(0.5),
    });
    e.send(Command::TogglePlay { deck: 0 });
    e.send(Command::TogglePlay { deck: 1 });
    let out = render(&mut r, 10);
    assert!(out.iter().all(|&x| x == 0.75));
    e.send(Command::Gain { deck: 1, gain: 0.5 });
    let out = render(&mut r, 10);
    assert!(out.iter().all(|&x| x == 0.5));
    // Mono and multichannel devices.
    let mut mono = [0.0; 4];
    r.render(&mut mono, 1);
    assert!(mono.iter().all(|&x| x == 0.5));
    let mut quad = [9.0; 8];
    r.render(&mut quad, 4);
    assert_eq!(quad, [0.5, 0.5, 0.0, 0.0, 0.5, 0.5, 0.0, 0.0]);
}

#[test]
fn speed_is_varispeed_on_the_same_step() {
    let (mut e, mut r) = setup(100_000);
    e.send(Command::Speed {
        deck: 0,
        speed: 1.5,
    });
    e.send(Command::TogglePlay { deck: 0 });
    render(&mut r, 1_000);
    assert_eq!(e.deck(0).frame, 1_500.0);
}

#[test]
fn commands_on_an_empty_or_unknown_deck_are_ignored() {
    let (mut e, mut r) = Engine::manual(RATE);
    for command in [
        Command::TogglePlay { deck: 0 },
        Command::CueDown { deck: 0 },
        Command::Seek {
            deck: 0,
            frame: 10.0,
        },
        Command::LoopIn { deck: 0 },
        Command::TogglePlay { deck: 5 },
    ] {
        e.send(command);
    }
    let out = render(&mut r, 32);
    assert!(out.iter().all(|&x| x == 0.0));
    assert_eq!(e.deck(0).id, None);
    assert!(!e.deck(0).playing);
    assert_eq!(e.deck(5), DeckSnapshot::default());
}

#[test]
fn pending_until_the_callback_applies_what_was_sent() {
    let (mut e, mut r) = setup(1_000);
    assert!(e.pending(0) && !e.pending(1));
    render(&mut r, 1);
    assert!(!e.pending(0));
    e.send(Command::Seek {
        deck: 0,
        frame: 10.0,
    });
    assert!(e.pending(0));
    render(&mut r, 1);
    assert!(!e.pending(0));
}

#[test]
fn full_ring_refuses_instead_of_blocking() {
    let (mut e, _r) = Engine::manual(RATE);
    for _ in 0..COMMAND_CAPACITY {
        assert!(e.send(Command::TogglePlay { deck: 0 }));
    }
    assert!(!e.send(Command::TogglePlay { deck: 0 }));
}

#[test]
fn replaced_track_is_freed_by_the_ui_never_by_the_callback() {
    let (mut e, mut r) = setup(1_000);
    render(&mut r, 1);
    let first = Arc::downgrade(&e.memory.current[0].as_ref().unwrap().1);
    e.send(Command::Load {
        deck: 0,
        id: 8,
        track: ramp(10, RATE),
    });
    // Before the callback swaps: UI (retired) + callback.
    e.poll();
    assert_eq!(first.strong_count(), 2);
    render(&mut r, 1);
    // Callback released its clone; the UI still holds one (no free there).
    assert_eq!(first.strong_count(), 1);
    e.poll();
    assert_eq!(first.strong_count(), 0);
    assert_eq!(e.deck(0).id, Some(8));
}

#[test]
fn device_loss_falls_back_to_silent_output_with_state_kept() {
    let (mut e, mut r) = setup(1_000_000);
    e.send(Command::Seek {
        deck: 0,
        frame: 4_000.0,
    });
    e.send(Command::CueDown { deck: 0 }); // cue = 4000
    e.send(Command::Gain { deck: 0, gain: 0.5 });
    e.send(Command::TogglePlay { deck: 0 });
    render(&mut r, 100);
    e.simulate_device_loss();
    e.poll();
    assert_eq!(e.output(), OutputKind::Silent);
    assert!(e.error().unwrap().contains("audio device lost"));
    // The silent clock keeps the deck moving from where it was.
    std::thread::sleep(std::time::Duration::from_millis(100));
    let s = e.deck(0);
    assert_eq!((s.id, s.cue), (Some(7), 4_000.0));
    assert!(s.playing && s.frame > 4_100.0, "{s:?}");
    assert_eq!(e.memory.gains[0], Some(0.5));
}

#[test]
fn silent_engine_reports_reason_and_plays_on() {
    let mut e = Engine::silent("no audio output device");
    assert_eq!(e.error(), Some("no audio output device"));
    assert_eq!(e.output(), OutputKind::Silent);
    e.send(Command::Load {
        deck: 0,
        id: 1,
        track: ramp(100_000, 44_100),
    });
    e.send(Command::TogglePlay { deck: 0 });
    std::thread::sleep(std::time::Duration::from_millis(150));
    assert!(e.deck(0).frame > 0.0);
}

#[test]
fn unload_stops_and_empties_one_deck_and_frees_the_track() {
    let (mut e, mut r) = setup(100_000);
    let b = ramp(100_000, RATE);
    assert!(e.send(Command::Load {
        deck: 1,
        id: 9,
        track: b
    }));
    e.send(Command::Gain { deck: 0, gain: 0.5 });
    e.send(Command::TogglePlay { deck: 0 });
    e.send(Command::TogglePlay { deck: 1 });
    render(&mut r, 1_000);
    assert!(e.deck(0).playing && e.deck(1).playing);
    assert!(e.send(Command::Unload { deck: 0 }));
    render(&mut r, 100);
    let a = e.deck(0);
    assert_eq!(
        (a.id, a.playing, a.frame, a.loop_range),
        (None, false, 0.0, None)
    );
    let b = e.deck(1);
    assert_eq!(
        (b.id, b.playing, b.frame),
        (Some(9), true, 1_100.0),
        "deck B unaffected"
    );
    // Play on the empty deck stays silent; only deck B is audible.
    e.send(Command::TogglePlay { deck: 0 });
    let out = render(&mut r, 4);
    assert!(!e.deck(0).playing);
    assert!(
        (frame_of(out[0]) - 1_100.0).abs() < 1e-2,
        "output is deck B alone: {}",
        out[0]
    );
    // The callback dropped its reference; poll frees the retired track.
    e.poll();
    assert!(e.retired.is_empty());
    // A new track loads normally and keeps the deck's gain.
    assert!(e.send(Command::Load {
        deck: 0,
        id: 8,
        track: ramp(10_000, RATE)
    }));
    e.send(Command::TogglePlay { deck: 1 });
    e.send(Command::Seek {
        deck: 0,
        frame: 2_000.0,
    });
    e.send(Command::TogglePlay { deck: 0 });
    let out = render(&mut r, 1);
    assert_eq!(e.deck(0).id, Some(8));
    assert!(
        (frame_of(out[0]) - 1_000.0).abs() < 1e-3,
        "gain 0.5 kept: {}",
        out[0]
    );
}

#[test]
fn output_config_reads_the_environment_keys() {
    let none = OutputConfig::from_lookup(|_| None);
    assert_eq!(none, OutputConfig::default());
    // Blank and unparsable values count as unset.
    let blank = OutputConfig::from_lookup(|k| match k {
        "OXIDJ_AUDIO_HOST" => Some("  ".into()),
        "OXIDJ_AUDIO_BUFFER" => Some("small".into()),
        _ => None,
    });
    assert_eq!(blank, OutputConfig::default());
    let full = OutputConfig::from_lookup(|k| {
        Some(
            match k {
                "OXIDJ_AUDIO_HOST" => "jack",
                "OXIDJ_AUDIO_DEVICE" => "hw:CARD=X,DEV=0",
                "OXIDJ_AUDIO_RATE" => "44100",
                "OXIDJ_AUDIO_BUFFER" => " 128 ",
                "OXIDJ_AUDIO_CHANNELS" => "2",
                _ => return None,
            }
            .to_owned(),
        )
    });
    assert_eq!(
        full,
        OutputConfig {
            host: Some("jack".into()),
            device: Some("hw:CARD=X,DEV=0".into()),
            sample_rate: Some(44_100),
            buffer_frames: Some(128),
            channels: Some(2),
            routing: Routing::default(),
        }
    );
    // A named device alone keeps the cue contract: 4 channels at 48 kHz.
    let named = OutputConfig::from_lookup(|k| (k == "OXIDJ_AUDIO_DEVICE").then(|| "x".into()));
    assert_eq!((named.sample_rate, named.channels), (Some(48_000), Some(4)));
}

#[test]
fn an_engine_without_a_device_never_retries_one() {
    let (mut e, _r) = setup(1_000);
    assert!(e.retry_at.is_none());
    e.simulate_device_loss();
    e.poll();
    assert_eq!(e.output(), OutputKind::Silent);
    assert!(e.retry_at.is_none());
    let s = Engine::silent("no audio in tests");
    assert!(s.retry_at.is_none());
    assert_eq!(s.config(), &OutputConfig::default());
}

#[test]
fn an_unknown_host_or_device_falls_back_silently_and_keeps_retrying() {
    let mut e = Engine::start_with(OutputConfig {
        host: Some("no-such-host".into()),
        ..OutputConfig::default()
    });
    assert_eq!(e.output(), OutputKind::Silent);
    assert!(e.error().unwrap().contains("audio host not available"));
    assert!(e.retry_at.is_some());
    // The decks survive a failed switch too.
    e.send(Command::Load {
        deck: 0,
        id: 9,
        track: ramp(480_000, RATE),
    });
    e.send(Command::Seek {
        deck: 0,
        frame: 1_234.0,
    });
    std::thread::sleep(std::time::Duration::from_millis(60));
    e.reopen(OutputConfig {
        host: Some("still-no-host".into()),
        buffer_frames: Some(128),
        ..OutputConfig::default()
    });
    assert_eq!(e.config().buffer_frames, Some(128));
    assert!(e.error().unwrap().contains("still-no-host"));
    std::thread::sleep(std::time::Duration::from_millis(60));
    let s = e.deck(0);
    assert_eq!((s.id, s.frame), (Some(9), 1_234.0));
}

/// A track of `frames` frames, every sample `value`.
fn flat(frames: usize, value: f32) -> Arc<TrackPcm> {
    Arc::new(TrackPcm::new(RATE, vec![value; frames * 2]).unwrap())
}

/// Largest jump between consecutive left samples (from silence before).
fn max_step(out: &[f32]) -> f32 {
    let mut last = 0.0;
    let mut worst = 0.0_f32;
    for f in out.as_chunks::<2>().0 {
        worst = worst.max((f[0] - last).abs());
        last = f[0];
    }
    worst
}

/// Deck 0 with a flat 0.5 track and a 3 ms (144 frame) declick.
fn declick_setup() -> (Engine, Renderer) {
    let (mut e, mut r) = Engine::manual(RATE);
    r.set_declick_ms(DECLICK_MS);
    e.send(Command::Load {
        deck: 0,
        id: 1,
        track: flat(200_000, 0.5),
    });
    (e, r)
}

#[test]
fn declick_ramps_play_and_pause_instead_of_stepping() {
    let (mut e, mut r) = declick_setup();
    e.send(Command::TogglePlay { deck: 0 });
    let start = render(&mut r, 400);
    // 144 frames of ramp: no step larger than one ramp increment.
    assert!(max_step(&start) <= 0.5 / 144.0, "{}", max_step(&start));
    assert!(start[0] > 0.0 && start[0] < 0.01);
    assert_eq!(start[2 * 144], 0.5);
    assert_eq!(e.deck(0).frame, 400.0, "the fade does not delay the deck");
    e.send(Command::TogglePlay { deck: 0 });
    let stop = render(&mut r, 400);
    assert_eq!(e.deck(0).frame, 400.0, "paused at once");
    assert!(!e.deck(0).playing);
    let mut ramp = vec![0.5, 0.5];
    ramp.extend_from_slice(&stop);
    assert!(
        max_step(&ramp[2..]) <= 0.5,
        "first sample is near full level"
    );
    assert!((stop[0] - 0.5).abs() < 0.01);
    assert!(
        stop.as_chunks::<2>()
            .0
            .windows(2)
            .all(|w| w[1][0] <= w[0][0])
    );
    assert!(
        stop.as_chunks::<2>()
            .0
            .windows(2)
            .all(|w| w[0][0] - w[1][0] <= 0.5 / 144.0)
    );
    assert_eq!(stop[2 * 144], 0.0);
    assert_eq!(stop[1], stop[0], "both channels fade alike");
}

#[test]
fn declick_crossfades_seeks_cue_jumps_and_loop_seams_at_constant_level() {
    let (mut e, mut r) = declick_setup();
    e.send(Command::TogglePlay { deck: 0 });
    render(&mut r, 1_000);
    let level = |out: &[f32], what: &str| {
        for (i, v) in out.iter().enumerate() {
            assert!((v - 0.5).abs() < 1e-5, "{what}: sample {i} = {v}");
        }
    };
    // Seek while playing: old and new overlap with gains summing to 1.
    e.send(Command::Seek {
        deck: 0,
        frame: 50_000.0,
    });
    level(&render(&mut r, 500), "seek");
    assert_eq!(e.deck(0).frame, 50_500.0);
    // A short loop: every seam crossfades.
    e.send(Command::LoopIn { deck: 0 });
    render(&mut r, 960);
    e.send(Command::LoopOut { deck: 0 });
    level(&render(&mut r, 5_000), "loop seam");
    let f = e.deck(0).frame;
    assert!((50_500.0..51_460.0).contains(&f), "{f}");
    // A fresh seek before the previous fade ended still never exceeds it.
    e.send(Command::LoopOut { deck: 0 });
    e.send(Command::Seek {
        deck: 0,
        frame: 10.0,
    });
    render(&mut r, 20);
    e.send(Command::Seek {
        deck: 0,
        frame: 90_000.0,
    });
    let out = render(&mut r, 400);
    assert!(out.iter().all(|v| (0.0..=0.5001).contains(v)));
    assert_eq!(out[2 * 200], 0.5);
}

#[test]
fn declick_fades_the_old_track_out_when_a_playing_deck_is_reloaded() {
    let (mut e, mut r) = declick_setup();
    e.send(Command::TogglePlay { deck: 0 });
    render(&mut r, 1_000);
    let next = flat(1_000, -0.25);
    e.send(Command::Load {
        deck: 0,
        id: 2,
        track: next,
    });
    let out = render(&mut r, 300);
    assert_eq!(e.deck(0).id, Some(2));
    assert!(!e.deck(0).playing);
    assert!((out[0] - 0.5).abs() < 0.01, "old track still sounding");
    assert!(out.iter().all(|v| (0.0..=0.5).contains(v)));
    assert_eq!(out[2 * 144], 0.0);
    // The old track is released once its tail has played out.
    e.poll();
    assert!(e.retired.is_empty());
}

#[test]
fn declick_on_a_ramp_bounds_the_step_at_a_jump() {
    let (mut e, mut r) = Engine::manual(RATE);
    r.set_declick_ms(DECLICK_MS);
    // Left sample = frame / 1e6: a seek of 500,000 frames is a 0.5 step.
    e.send(Command::Load {
        deck: 0,
        id: 1,
        track: ramp(1_000_000, RATE),
    });
    e.send(Command::Seek {
        deck: 0,
        frame: 100_000.0,
    });
    e.send(Command::TogglePlay { deck: 0 });
    render(&mut r, 1_000);
    e.send(Command::Seek {
        deck: 0,
        frame: 600_000.0,
    });
    let out = render(&mut r, 400);
    let mut joined = vec![0.100_999, 0.0];
    joined.extend_from_slice(&out);
    assert!(max_step(&joined[2..]) < 0.11, "start level");
    let steps: f32 = out
        .as_chunks::<2>()
        .0
        .windows(2)
        .map(|w| (w[1][0] - w[0][0]).abs())
        .fold(0.0, f32::max);
    assert!(steps < 0.5 / 140.0, "largest step {steps}");
    assert!((frame_of(out[2 * 399]) - 600_399.0).abs() < 1.0);
}

/// Both decks playing a 1 kHz sine of peak `amp` in phase (sum = 2 × amp).
fn hot_mix(amp: f32) -> (Engine, Renderer) {
    let (mut e, r) = Engine::manual(RATE);
    for deck in 0..2 {
        let samples = (0..RATE as usize * 2)
            .flat_map(|i| {
                let v = (i as f32 * 1_000.0 * std::f32::consts::TAU / RATE as f32).sin() * amp;
                [v, v]
            })
            .collect();
        e.send(Command::Load {
            deck,
            id: deck as u64,
            track: Arc::new(TrackPcm::new(RATE, samples).unwrap()),
        });
        e.send(Command::TogglePlay { deck });
    }
    (e, r)
}

/// Samples whose magnitude sits on the clamp (a flat-topped wave).
fn flat_tops(out: &[f32]) -> usize {
    out.iter().filter(|v| v.abs() >= 0.9999).count()
}

#[test]
fn limiter_turns_overs_down_instead_of_clipping_them() {
    // 2 × 0.8 = 1.6 peak: +4 dB over full scale.
    let (mut e, mut r) = hot_mix(0.8);
    let out = render(&mut r, 9_600);
    assert!(out.iter().all(|v| v.abs() <= 1.0));
    // After the first peak the wave keeps its shape: per 48-frame cycle at
    // most the two crests touch full scale (clipping flattens about 40 %).
    let settled = &out[2 * 4_800..];
    assert!(flat_tops(settled) <= 4 * 100 + 8, "{}", flat_tops(settled));
    let rms =
        (settled.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / settled.len() as f64).sqrt();
    // A sine at full scale has RMS 0.707; a clipped 1.6 sine is far above.
    assert!((0.66..0.72).contains(&rms), "rms {rms}");

    // Limiter off: the same mix is clipped flat.
    e.send(Command::Limiter(false));
    let clipped = render(&mut r, 9_600);
    assert!(flat_tops(&clipped[2 * 4_800..]) > 4_800 * 2 / 3);
    // On again, it recovers within the release time once the mix is quiet.
    e.send(Command::Limiter(true));
    render(&mut r, 4_800);
    for deck in 0..2 {
        e.send(Command::Gain { deck, gain: 0.25 });
    }
    render(&mut r, 48_000);
    let quiet = render(&mut r, 480);
    let peak = quiet.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
    assert!((peak - 0.4).abs() < 0.001, "full gain is back: {peak}");
}

#[test]
fn limiter_leaves_a_mix_below_full_scale_bit_identical() {
    let (mut on, mut r_on) = hot_mix(0.45);
    let (mut off, mut r_off) = hot_mix(0.45);
    off.send(Command::Limiter(false));
    assert_eq!(render(&mut r_on, 4_800), render(&mut r_off, 4_800));
    // Not a deck command: nothing is left pending.
    assert!(!on.pending(0) || on.deck(0).applied > 0);
    on.send(Command::Limiter(true));
    render(&mut r_on, 16);
    assert!(!on.pending(0) && !on.pending(1));
}

#[test]
fn declick_covers_a_key_lock_switch_at_the_same_position() {
    let (mut e, mut r) = declick_setup();
    e.send(Command::TogglePlay { deck: 0 });
    render(&mut r, 1_000);
    e.send(Command::KeyLock { deck: 0, on: true });
    let out = render(&mut r, 400);
    // The plain signal fades out from full level instead of stopping dead
    // while the stretch engine starts underneath it.
    assert!((out[0] - 0.5).abs() < 0.05, "{}", out[0]);
    let worst = out[..2 * 100]
        .as_chunks::<2>()
        .0
        .windows(2)
        .map(|w| (w[1][0] - w[0][0]).abs())
        .fold(0.0, f32::max);
    assert!(worst < 0.05, "largest step {worst}");
    assert!(e.deck(0).keylock);
}

/// Largest jump between consecutive left samples of `out`, starting from
/// `last` (the sample rendered just before it).
fn worst_step(last: f32, out: &[f32]) -> f32 {
    let mut last = last;
    let mut worst = 0.0_f32;
    for f in out.as_chunks::<2>().0 {
        worst = worst.max((f[0] - last).abs());
        last = f[0];
    }
    worst
}

/// Deck 0 playing a staircase track (left = 0.1 per 100,000 frames), so a
/// seek between stairs is a visible step unless it is faded.
fn stairs_setup() -> (Engine, Renderer) {
    let (mut e, mut r) = Engine::manual(RATE);
    r.set_declick_ms(DECLICK_MS);
    let samples = (0..900_000)
        .flat_map(|i| {
            let v = (i / 100_000) as f32 * 0.1;
            [v, v]
        })
        .collect();
    e.send(Command::Load {
        deck: 0,
        id: 1,
        track: Arc::new(TrackPcm::new(RATE, samples).unwrap()),
    });
    e.send(Command::Seek {
        deck: 0,
        frame: 850_000.0,
    });
    e.send(Command::TogglePlay { deck: 0 });
    render(&mut r, 1_000);
    (e, r)
}

#[test]
fn declick_two_jumps_in_one_callback_fade_from_what_was_heard() {
    let (mut e, mut r) = stairs_setup(); // sounding at level 0.8
    // Scrub: two seeks land in the same drain. Level 0.1 was never heard.
    e.send(Command::Seek {
        deck: 0,
        frame: 150_000.0,
    });
    e.send(Command::Seek {
        deck: 0,
        frame: 450_000.0,
    });
    let out = render(&mut r, 400);
    // One smooth crossfade 0.8 -> 0.4: no step beyond a ramp increment.
    assert!(
        worst_step(0.8, &out) < 0.8 / 140.0,
        "{}",
        worst_step(0.8, &out)
    );
    assert!((out[2 * 399] - 0.4).abs() < 1e-6);
}

#[test]
fn declick_a_jump_in_the_middle_of_a_fade_never_steps_by_more_than_half() {
    for into_fade in [10, 40, 72, 100, 130] {
        let (mut e, mut r) = stairs_setup(); // 0.8
        e.send(Command::Seek {
            deck: 0,
            frame: 50_000.0, // level 0.0: the fade goes 0.8 -> 0.0
        });
        let first = render(&mut r, into_fade);
        let last = first[2 * (into_fade - 1)];
        e.send(Command::Seek {
            deck: 0,
            frame: 450_000.0, // 0.4
        });
        let out = render(&mut r, 400);
        let step = worst_step(last, &out);
        // Without the rule the splice is the full 0.8 (old tail cut and a
        // new one started at full gain); now at most half of it.
        assert!(step <= 0.4 + 0.01, "{into_fade} frames in: step {step}");
        assert!((out[2 * 399] - 0.4).abs() < 1e-6);
    }
}

#[test]
fn declick_gives_a_key_locked_deck_no_mismatched_tail() {
    let (mut e, mut r) = declick_setup();
    e.send(Command::KeyLock { deck: 0, on: true });
    e.send(Command::Speed {
        deck: 0,
        speed: 1.08,
    });
    e.send(Command::TogglePlay { deck: 0 });
    render(&mut r, 20_000);
    e.send(Command::TogglePlay { deck: 0 });
    // The stretch output stops; no plain-read tail is laid over it.
    assert!(render(&mut r, 300).iter().all(|v| *v == 0.0));
}

#[test]
fn reopen_keeps_a_load_the_callback_had_not_applied_yet() {
    let mut e = Engine::silent("no audio in tests");
    e.send(Command::Load {
        deck: 0,
        id: 1,
        track: ramp(480_000, RATE),
    });
    e.send(Command::TogglePlay { deck: 0 });
    std::thread::sleep(std::time::Duration::from_millis(60));
    assert_eq!(e.deck(0).id, Some(1));
    // A new track goes into the ring and the output is replaced at once.
    e.send(Command::Load {
        deck: 0,
        id: 2,
        track: ramp(480_000, RATE),
    });
    e.reopen(OutputConfig {
        host: Some("no-such-host".into()),
        ..OutputConfig::default()
    });
    std::thread::sleep(std::time::Duration::from_millis(60));
    let s = e.deck(0);
    assert!(s.id == Some(2) && !s.playing && s.frame == 0.0, "{s:?}");
    assert!(!e.pending(0));
}

#[test]
fn send_refuses_unknown_decks_and_remembers_only_valid_settings() {
    let (mut e, mut r) = setup(1_000);
    let stray = ramp(100, RATE);
    assert!(!e.send(Command::Load {
        deck: DECKS,
        id: 3,
        track: stray.clone(),
    }));
    render(&mut r, 16);
    assert_eq!(Arc::strong_count(&stray), 1, "never entered the ring");
    assert!(e.send(Command::Mixer(None)) && e.send(Command::Limiter(true)));
    e.send(Command::Gain { deck: 0, gain: 0.5 });
    e.send(Command::Gain {
        deck: 0,
        gain: f32::NAN,
    });
    e.send(Command::Speed {
        deck: 0,
        speed: 1.04,
    });
    e.send(Command::Speed {
        deck: 0,
        speed: -1.0,
    });
    e.send(Command::Sync {
        deck: 0,
        master: 1,
        speed: f64::NAN,
        beat: 1.0,
        anchor: 0.0,
        master_beat: 1.0,
        master_anchor: 0.0,
    });
    assert_eq!(e.memory.gains[0], Some(0.5));
    assert_eq!(e.memory.speeds[0], Some(1.04));
}

#[test]
fn limiter_look_ahead_delays_the_mix_and_bends_the_gain_before_the_peak() {
    // Quiet mix: only the 1 ms (48 frame) delay shows, bit for bit.
    let (_e, mut plain) = hot_mix(0.45);
    let (_e2, mut ahead) = hot_mix(0.45);
    ahead.set_limiter_lookahead_ms(LIMITER_LOOKAHEAD_MS);
    let a = render(&mut plain, 2_000);
    let b = render(&mut ahead, 2_000);
    assert!(b[..2 * 48].iter().all(|v| *v == 0.0));
    assert_eq!(a[..2 * 1_952], b[2 * 48..]);

    // Hot mix (peak 1.6): the delayed input divided into the output is the
    // gain. It never jumps, it is already down when the first over comes
    // out, and nothing reaches the clamp.
    let (_e, mut dry) = hot_mix(0.8);
    let (mut e, mut wet) = hot_mix(0.8);
    e.send(Command::Limiter(true));
    wet.set_limiter_lookahead_ms(LIMITER_LOOKAHEAD_MS);
    // Reference without any limiting: the raw sum is rebuilt from one deck.
    let (mut one, mut single) = hot_mix(0.8);
    one.send(Command::Gain { deck: 1, gain: 0.0 });
    let half = render(&mut single, 4_800);
    let out = render(&mut wet, 4_800);
    let _ = render(&mut dry, 1);
    // The gain is first seen when the delayed signal appears (frame 48).
    let mut last_gain = None;
    let mut worst_jump = 0.0_f32;
    let mut lowest = 1.0_f32;
    for n in 48..4_800 {
        let input = half[2 * (n - 48)] * 2.0;
        assert!(out[2 * n].abs() <= 1.0 + 1e-6, "frame {n}: {}", out[2 * n]);
        if input.abs() > 0.2 {
            let gain = out[2 * n] / input;
            worst_jump = worst_jump.max((gain - last_gain.unwrap_or(gain)).abs());
            lowest = lowest.min(gain);
            last_gain = Some(gain);
        }
    }
    // The first over needs 1/1.6 = 0.625; ramped over 48 frames that is
    // under 0.01 per frame. The instant limiter jumps by 0.2 and more.
    assert!(worst_jump < 0.03, "largest gain jump {worst_jump}");
    assert!((lowest - 0.625).abs() < 0.01, "lowest gain {lowest}");
    assert!(flat_tops(&out) <= 2 * 2 * 100, "{}", flat_tops(&out));
}

/// Deck 0 (0.25, fader down, CUE on) is heard on the cue bus only; deck 1
/// (0.5) on main only.
fn routed(routing: Routing, channels: usize) -> Vec<f32> {
    let (mut e, mut r) = Engine::manual(RATE);
    r.set_routing(routing);
    let mut params = MixerParams {
        cue_mix: 0.0,
        ..MixerParams::default()
    };
    params.channels[0].cue = true;
    params.channels[0].fader = 0.0;
    e.send(Command::Mixer(Some(params)));
    for (deck, level) in [(0, 0.25), (1, 0.5)] {
        e.send(Command::Load {
            deck,
            id: deck as u64,
            track: flat(48_000, level),
        });
        e.send(Command::TogglePlay { deck });
    }
    let mut out = vec![9.0; 4_000 * channels];
    r.render(&mut out, channels);
    // The settled last frame (filters and ramps have run in).
    out[out.len() - channels..].to_vec()
}

fn near(got: &[f32], want: &[f32]) {
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() < 1e-3, "{got:?} != {want:?}");
    }
}

#[test]
fn routing_places_main_and_cue_on_the_chosen_pairs() {
    let default = Routing::default();
    near(&routed(default, 4), &[0.5, 0.5, 0.25, 0.25]);
    near(&routed(default, 2), &[0.5, 0.5]);
    near(&routed(default, 3), &[0.5, 0.5, 0.0]);
    near(&routed(default, 6), &[0.5, 0.5, 0.25, 0.25, 0.0, 0.0]);
    // Swapped pairs, and pairs further up an 8-channel card.
    let swapped = Routing {
        master: 2,
        cue: 0,
        split: false,
    };
    near(&routed(swapped, 4), &[0.25, 0.25, 0.5, 0.5]);
    let high = Routing {
        master: 4,
        cue: 6,
        split: false,
    };
    near(
        &routed(high, 8),
        &[0.0, 0.0, 0.0, 0.0, 0.5, 0.5, 0.25, 0.25],
    );
    // A pair the device lacks stays silent; overlapping pairs give main.
    near(&routed(high, 4), &[0.0; 4]);
    let overlap = Routing {
        master: 0,
        cue: 1,
        split: false,
    };
    near(&routed(overlap, 4), &[0.5, 0.5, 0.0, 0.0]);
}

#[test]
fn split_cue_puts_mono_main_left_and_mono_cue_right() {
    let split = Routing {
        split: true,
        ..Routing::default()
    };
    near(&routed(split, 2), &[0.5, 0.25]);
    near(&routed(split, 4), &[0.5, 0.25, 0.0, 0.0]);
    // Mono devices carry main only, as before.
    near(&routed(split, 1), &[0.5]);
}

#[test]
fn routing_is_checked_against_the_device_and_read_from_the_environment() {
    let ok = Routing::default();
    assert!(ok.check(2).is_ok() && ok.check(4).is_ok() && ok.check(1).is_ok());
    let high = Routing {
        master: 4,
        cue: 6,
        split: false,
    };
    assert!(high.check(8).is_ok());
    assert!(
        high.check(4)
            .unwrap_err()
            .contains("main outputs 5/6 do not exist")
    );
    let overlap = Routing {
        master: 0,
        cue: 1,
        split: false,
    };
    assert!(overlap.check(4).unwrap_err().contains("overlap"));
    let split = Routing { split: true, ..ok };
    assert!(split.check(2).is_ok());
    assert!(split.check(1).unwrap_err().contains("split cue"));

    let env = OutputConfig::from_lookup(|k| {
        Some(
            match k {
                "OXIDJ_AUDIO_MASTER" => "3",
                "OXIDJ_AUDIO_CUE" => "1",
                _ => return None,
            }
            .to_owned(),
        )
    });
    assert_eq!(
        (env.routing.master, env.routing.cue, env.routing.split),
        (2, 0, false)
    );
    assert_eq!(env.channels, None);
    let env = OutputConfig::from_lookup(|k| match k {
        "OXIDJ_AUDIO_SPLIT_CUE" => Some("yes".into()),
        "OXIDJ_AUDIO_DEVICE" => Some("card".into()),
        "OXIDJ_AUDIO_MASTER" => Some("0".into()),
        _ => None,
    });
    assert!(env.routing.split);
    assert_eq!(env.channels, Some(2), "split cue asks for a stereo output");
    assert_eq!(
        env.routing.master, 0,
        "output 0 does not exist: default kept"
    );
}
