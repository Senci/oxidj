//! Production renderer bus isolation, including the exact device channel layout.
use cpal as _;
use oxidj_audio::{Command, Engine, MixerParams, Renderer, TrackPcm};
use rtrb as _;
use signalsmith_stretch as _;
use std::sync::Arc;

fn output(params: Option<MixerParams>, channels: usize) -> Vec<f32> {
    let (mut e, mut r) = Engine::manual(48_000);
    e.send(Command::Mixer(params));
    // Different constant stereo samples identify A/B and L/R independently.
    for (deck, stereo) in [(0, [0.2, 0.1]), (1, [0.4, 0.3])] {
        e.send(Command::Load {
            deck,
            id: deck as u64,
            track: Arc::new(TrackPcm::new(48_000, stereo.repeat(4096)).unwrap()),
        });
        e.send(Command::TogglePlay { deck });
    }
    let mut out = vec![0.0; 2048 * channels];
    r.render(&mut out, channels);
    out
}

fn approx(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-5, "{a} != {b}");
}

/// Interleaved stereo 440 Hz sine at `amp` peak.
fn sine(amp: f32, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|i| amp * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48_000.0).sin())
        .flat_map(|s| [s, s])
        .collect()
}

/// Render `frames` frames with per-deck stereo PCM (`L R` interleaved).
fn render(
    params: Option<MixerParams>,
    channels: usize,
    tracks: [Vec<f32>; 2],
    frames: usize,
) -> Vec<f32> {
    let (mut e, mut r) = Engine::manual(48_000);
    e.send(Command::Mixer(params));
    for (deck, pcm) in tracks.into_iter().enumerate() {
        e.send(Command::Load {
            deck,
            id: deck as u64,
            track: Arc::new(TrackPcm::new(48_000, pcm).unwrap()),
        });
        e.send(Command::TogglePlay { deck });
    }
    let mut out = vec![0.0; frames * channels];
    r.render(&mut out, channels);
    out
}

/// Render `chunks` whole 256-frame blocks into one interleaved buffer.
fn render_chunks(r: &mut Renderer, buf: &mut [f32], chunks: usize) -> Vec<f32> {
    let mut out = Vec::new();
    for _ in 0..chunks {
        r.render(buf, 4);
        out.extend_from_slice(buf);
    }
    out
}

/// The last frame of each of the final `chunks` 256-frame blocks.
fn tail_last_frames(out: &[f32], channels: usize, chunks: usize) -> Vec<Vec<f32>> {
    let frames = out.len() / channels;
    (0..chunks)
        .map(|k| {
            let f = frames - 1 - k * 256;
            out[f * channels..(f + 1) * channels].to_vec()
        })
        .collect()
}

#[test]
fn cue_toggle_mix_and_volume_reach_only_headphones() {
    let base = MixerParams::default();
    let baseline = output(Some(base), 4);
    for cues in [[false, false], [true, false], [false, true], [true, true]] {
        for mix in [0.0, 0.25, 0.5, 1.0] {
            for volume in [0.0, 0.5, 1.0] {
                let mut p = base;
                for (ch, cue) in p.channels.iter_mut().zip(cues) {
                    ch.cue = cue;
                }
                p.cue_mix = mix;
                p.cue_volume = volume;
                let out = output(Some(p), 4);
                for (actual, expected) in out
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(baseline.as_chunks::<4>().0.iter())
                {
                    assert_eq!(
                        &actual[..2],
                        &expected[..2],
                        "cue must not alter main, even during DSP startup"
                    );
                }
                // The settled cue level is asserted at the end of each of the
                // last four chunks, not only in the final frame.
                for frame in tail_last_frames(&out, 4, 4) {
                    for (i, (a, b)) in [(0.2, 0.4), (0.1, 0.3)].into_iter().enumerate() {
                        let expected = ((if cues[0] { a * (1.0 - mix) } else { 0.0 })
                            + (if cues[1] { b * mix } else { 0.0 }))
                            * volume;
                        approx(frame[2 + i], expected);
                    }
                }
            }
        }
    }
}

#[test]
fn cue_is_pre_fader_crossfader_and_master_and_extra_channels_are_silent() {
    let mut p = MixerParams {
        master: 0.0,
        crossfader: Some(1.0),
        cue_mix: 0.0,
        ..Default::default()
    };
    p.channels[0].fader = 0.0;
    p.channels[0].cue = true;
    let out = output(Some(p), 6);
    for frame in tail_last_frames(&out, 6, 4) {
        assert_eq!(&frame[..2], &[0.0, 0.0], "main must stay silent");
        approx(frame[2], 0.2);
        approx(frame[3], 0.1);
        assert_eq!(&frame[4..], &[0.0, 0.0], "channels 5+ must stay silent");
    }
    // No partial mono cue on three-channel devices, no cue folded into stereo.
    for channels in [1, 2, 3] {
        let off = output(
            Some(MixerParams {
                channels: p.channels.map(|mut c| {
                    c.cue = false;
                    c
                }),
                ..p
            }),
            channels,
        );
        assert_eq!(output(Some(p), channels), off);
    }
    let hidden = output(None, 4);
    assert!(
        hidden
            .as_chunks::<4>()
            .0
            .iter()
            .all(|f| f[2..] == [0.0, 0.0])
    );
}

#[test]
fn cue_parameters_clamp_and_nan_cannot_reach_output() {
    for (mix, expected) in [(-1.0, [0.2, 0.1]), (2.0, [0.4, 0.3])] {
        let mut p = MixerParams {
            cue_mix: mix,
            cue_volume: 2.0,
            ..Default::default()
        };
        p.channels.iter_mut().for_each(|c| c.cue = true);
        let out = output(Some(p), 4);
        approx(out[out.len() - 2], expected[0]);
        approx(out[out.len() - 1], expected[1]);
        p.cue_volume = -1.0;
        assert!(
            output(Some(p), 4)
                .as_chunks::<4>()
                .0
                .iter()
                .all(|f| f[2..] == [0.0, 0.0])
        );
        p.cue_volume = f32::NAN;
        assert!(
            output(Some(p), 4)
                .as_chunks::<4>()
                .0
                .iter()
                .all(|f| f[2..] == [0.0, 0.0])
        );
        p.cue_mix = f32::NAN;
        p.cue_volume = 1.0;
        assert!(
            output(Some(p), 4)
                .as_chunks::<4>()
                .0
                .iter()
                .all(|f| f[2..] == [0.0, 0.0])
        );
    }
}

/// The cue tap is the post-filter, pre-fader signal: with the channel fader at
/// unity, the crossfader hard over and the master at unity, the headphone bus
/// carries exactly what the main bus carries, so the colour filter shapes both.
#[test]
fn cue_tap_follows_the_colour_filter() {
    const FRAMES: usize = 4096;
    let tone = sine(0.2, FRAMES);
    let silent = vec![0.0; FRAMES * 2];
    let mut p = MixerParams {
        crossfader: Some(0.0),
        cue_mix: 0.0,
        ..Default::default()
    };
    p.channels[0].cue = true;

    let open = render(Some(p), 4, [tone.clone(), silent.clone()], FRAMES);
    let mut filtered = p;
    filtered.channels[0].filter = -1.0; // 120 Hz low-pass
    let shut = render(Some(filtered), 4, [tone.clone(), silent.clone()], FRAMES);

    for (out, label) in [(&open, "filter open"), (&shut, "filter closed")] {
        for frame in tail_last_frames(out, 4, 4) {
            approx(frame[0], frame[2]);
            approx(frame[1], frame[3]);
            assert!(frame[2].abs() > 0.001, "{label}: cue bus silent: {frame:?}");
        }
    }
    // Peak over the final chunk: the filter shapes the cue exactly as it
    // shapes the main bus, so the tap is after the colour filter.
    let peak = |out: &[f32]| {
        let frames = out.len() / 4;
        out[(frames - 256) * 4..]
            .as_chunks::<4>()
            .0
            .iter()
            .map(|f| f[2].abs())
            .fold(0.0, f32::max)
    };
    let open_peak = peak(&open);
    let shut_peak = peak(&shut);
    assert!(open_peak > 0.15, "filter open: cue bus silent: {open_peak}");
    assert!(
        shut_peak < open_peak * 0.25,
        "cue must follow the colour filter: open {open_peak}, shut {shut_peak}"
    );
}

/// CUE switches and MIX moves ramp the per-deck cue gain across a chunk, so the
/// headphone bus fades instead of clicking.
#[test]
fn cue_gain_changes_ramp_without_a_click() {
    let (mut e, mut r) = Engine::manual(48_000);
    e.send(Command::Load {
        deck: 0,
        id: 0,
        track: Arc::new(TrackPcm::new(48_000, sine(0.2, 4096)).unwrap()),
    });
    e.send(Command::TogglePlay { deck: 0 });
    let mut p = MixerParams {
        crossfader: Some(0.0),
        cue_mix: 0.0,
        ..Default::default()
    };
    e.send(Command::Mixer(Some(p)));

    let mut buf = vec![0.0; 256 * 4];
    // CUE off: the bus is silent.
    let mut cue = render_chunks(&mut r, &mut buf, 4);
    // CUE on: the bus ramps up over the first chunk.
    p.channels[0].cue = true;
    e.send(Command::Mixer(Some(p)));
    cue.extend(render_chunks(&mut r, &mut buf, 4));
    // MIX to B (deck B is empty): the bus ramps back down to silence.
    p.cue_mix = 1.0;
    e.send(Command::Mixer(Some(p)));
    cue.extend(render_chunks(&mut r, &mut buf, 4));

    let frames = cue.as_chunks::<4>().0;
    let phase_on = &frames[4 * 256..8 * 256];
    let ramp_peak = phase_on[..64]
        .iter()
        .map(|f| f[2].abs())
        .fold(0.0, f32::max);
    assert!(
        ramp_peak < 0.12,
        "cue jumps to full level on the CUE chunk: {ramp_peak}"
    );
    let full_peak = phase_on[256 - 64..]
        .iter()
        .map(|f| f[2].abs())
        .fold(0.0, f32::max);
    assert!(
        full_peak > 0.15,
        "cue never reaches the post-filter signal: {full_peak}"
    );
    let tail_peak = frames[11 * 256..]
        .iter()
        .map(|f| f[2].abs())
        .fold(0.0, f32::max);
    assert!(tail_peak < 1e-6, "cue must fade to silence: {tail_peak}");

    // No sample-to-sample step may exceed the signal's own slope by much.
    let natural = 0.2 * 2.0 * (std::f32::consts::PI * 440.0 / 48_000.0).sin();
    let max_step = frames
        .windows(2)
        .map(|w| (w[1][2] - w[0][2]).abs())
        .fold(0.0, f32::max);
    assert!(
        max_step < natural * 2.5,
        "cue gain change clicks: max step {max_step}, signal step {natural}"
    );
}
