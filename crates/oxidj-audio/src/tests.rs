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
