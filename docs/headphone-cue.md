# Headphone cue output (M8 follow-on)

The mixer CUE buttons are headphone routing switches, **not** the transport CUE
buttons. A lit/selected switch sends that deck's post-trim/EQ/filter,
pre-channel-fader signal to the independent cue bus. The crossfader and main
master volume never attenuate this bus. MIX blends cued A (0) to cued B (1),
with a linear 50/50 blend at the centre. An unselected deck contributes silence;
there is no automatic gain compensation when only one deck is cued. CUE volume
scales the resulting stereo bus once. Both knobs clamp to 0–1; double-click
resets MIX to 0.5 and CUE volume to 1.0. Native egui focus, arrows, wheel,
AccessKit value actions and selected CUE state are regression-tested.

## Device layout and selection

- Outputs 1–2: main L/R, unchanged by cue controls.
- Outputs 3–4: headphone cue L/R on a device exposing a complete pair.
- Outputs 5 onward: silence. Mono/stereo/three-channel outputs never receive cue.
- Mixer hidden: exact main unity bypass and silent cue, as before.

This MVP uses **one multichannel device**, not two unsynchronised stereo devices.
Connect headphones to that interface's outputs 3–4; a stereo-only laptop output
cannot play the separate cue bus. Do not connect outputs 3–4 to main speakers.
The normal launch still opens the OS default device/configuration. To choose a
four-channel endpoint explicitly (48 kHz), enumerate and select its exact,
unambiguous CPAL description:

```sh
cargo run -p oxidj-audio --example cue_output -- --list
OXIDJ_AUDIO_DEVICE='Your four-channel interface description' oxidj-library
```

An unavailable, ambiguous or unsupported selection fails closed to the existing
paced silent output with a visible engine error. It never falls back to a
different physical output and never changes the OS default. Selection is a
startup environment option, not a hot-switching device picker. Normal default
output retains its existing configuration/rate.

## Reproducible virtual endpoint check

```sh
python3 scripts/test-cue-pipewire.py
```

Linux prerequisites: PipeWire's ALSA plugin, `pactl`, `parec`, Python 3, Cargo.
The script creates a uniquely named four-channel null sink, directs the real
production CPAL stream to it with `OXIDJ_AUDIO_DEVICE='PipeWire Sound Server'`
and `PIPEWIRE_NODE=<temporary sink>`, and records that sink's monitor. It sends
synthetic A=440 Hz to headphones only and B=880 Hz to main only, with distinct
left/right levels. It asserts channel RMS within 5% and unwanted frequency
amplitude below −60 dB, checks the default sink unchanged and unloads the
module in `finally`. No private library or audio is used.

Observed during implementation: 471 callbacks, 0 reported xruns/errors;
main L/R RMS 0.070697/0.035342, cue L/R 0.070697/0.035341; frequency isolation
passed. The test also caught and fixed inherited stereo duplication and a
squared cue-volume gain. Manual-renderer regressions cover all cue switches,
blend positions, half-volume linear scaling, clamping, NaN, main invariance,
pre-fader/master isolation, bypass and incomplete/extra channel layouts.
The allocation harness exercises four-channel cue while controls change.

**Not demonstrated:** physical headphone/interface routing, ASIO, macOS,
platform/device channel-map conventions, latency or long-run reliability.
These checks do not qualify club hardware or two-device synchronization.
Independent review and post-approval installation/publication remain separate
gates; the artwork WIP in the preceding commit is owned by its own card.
