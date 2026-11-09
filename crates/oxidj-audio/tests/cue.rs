//! Production renderer bus isolation, including the exact device channel layout.
use cpal as _;
use oxidj_audio::{Command, Engine, MixerParams, TrackPcm};
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
                let last = &out[out.len() - 4..];
                for (i, (a, b)) in [(0.2, 0.4), (0.1, 0.3)].into_iter().enumerate() {
                    let expected = ((if cues[0] { a * (1.0 - mix) } else { 0.0 })
                        + (if cues[1] { b * mix } else { 0.0 }))
                        * volume;
                    approx(last[2 + i], expected);
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
    let last = &out[out.len() - 6..];
    assert_eq!(&last[..2], &[0.0, 0.0]);
    approx(last[2], 0.2);
    approx(last[3], 0.1);
    assert_eq!(&last[4..], &[0.0, 0.0]);
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
