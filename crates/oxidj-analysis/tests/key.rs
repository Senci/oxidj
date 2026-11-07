//! Synthetic independent fixtures for the chroma/key estimator.
use oxidj_analysis::key::{Key, KeyBuilder};
use oxidj_core as _;
use taktus as _;

const RATE: u32 = 8_000;

fn chord_progression(root: u8, minor: bool) -> Vec<f32> {
    let mut pcm = Vec::new();
    // Three repeated triads, each held for 1.5 s; exact equal-tempered pitches.
    for chord in 0..3 {
        let root_midi = 48 + root + [0, 5, 7][chord];
        let intervals = if minor { [0, 3, 7] } else { [0, 4, 7] };
        for i in 0..(RATE as usize * 3 / 2) {
            let t = i as f32 / RATE as f32;
            let sample = intervals
                .iter()
                .map(|interval| {
                    let midi = root_midi + interval;
                    let hz = 440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0);
                    (std::f32::consts::TAU * hz * t).sin() * 0.2
                })
                .sum::<f32>();
            pcm.push(sample);
        }
    }
    pcm
}

fn analyze(pcm: &[f32], chunk: usize) -> Option<Key> {
    let mut builder = KeyBuilder::new(RATE);
    for part in pcm.chunks(chunk) {
        builder.push(part);
    }
    builder.finish()
}

#[test]
fn synthetic_c_major_fixture_matches_independent_expected_key() {
    assert_eq!(
        analyze(&chord_progression(0, false), 137),
        Some(Key::Major(0))
    );
}

#[test]
fn synthetic_a_minor_fixture_matches_independent_expected_key() {
    assert_eq!(
        analyze(&chord_progression(9, true), 251),
        Some(Key::Minor(9))
    );
}

#[test]
fn degenerate_dc_and_broadband_noise_do_not_invent_a_key() {
    assert_eq!(analyze(&vec![0.4; RATE as usize * 6], 257), None);
    let mut state = 0xDEAD_BEEF_1234_5678u64;
    let noise: Vec<f32> = (0..RATE as usize * 10)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            ((state >> 40) as f32 / 8_388_607.5) - 1.0
        })
        .collect();
    assert_eq!(analyze(&noise, 509), None);
}

#[test]
fn empty_silence_and_non_finite_input_do_not_invent_a_key() {
    assert_eq!(analyze(&[], 64), None);
    assert_eq!(analyze(&vec![0.0; RATE as usize * 6], 257), None);
    let mut bad = KeyBuilder::new(RATE);
    bad.push(&[0.0, f32::NAN]);
    assert_eq!(bad.finish(), None);
}
