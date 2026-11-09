#!/usr/bin/env python3
"""Linux-only, opt-in real CPAL → temporary PipeWire sink → monitor test.
Requires pactl, parec, PipeWire ALSA, cargo. Never changes the default sink.
Run: python3 scripts/test-cue-pipewire.py
"""
import array
import json
import math
import os
import subprocess as sp
import tempfile
import time


def command(*args):
    return sp.check_output(args, text=True).strip()


def main():
    sp.run(['cargo', 'build', '-p', 'oxidj-audio', '--example', 'cue_output'], check=True)
    before = command('pactl', 'get-default-sink')
    sink = f'oxidj_cue_test_{os.getpid()}'
    module = command('pactl', 'load-module', 'module-null-sink', f'sink_name={sink}',
                     'channels=4', 'channel_map=front-left,front-right,rear-left,rear-right', 'rate=48000')
    recorder = None
    try:
        with tempfile.TemporaryFile() as captured:
            recorder = sp.Popen(['parec', '--raw', f'--device={sink}.monitor', '--format=float32le',
                                 '--rate=48000', '--channels=4',
                                 '--channel-map=front-left,front-right,rear-left,rear-right'], stdout=captured)
            time.sleep(0.4)
            env = dict(os.environ, OXIDJ_AUDIO_DEVICE='PipeWire Sound Server', PIPEWIRE_NODE=sink)
            result = sp.run(['target/debug/examples/cue_output'], env=env, capture_output=True, text=True, timeout=20)
            print(result.stdout.strip())
            if result.returncode:
                raise RuntimeError(result.stderr)
            recorder.terminate()
            recorder.wait(timeout=5)
            recorder = None
            captured.seek(0)
            samples = array.array('f')
            samples.frombytes(captured.read())
        # Two seconds within playback, away from stream/DSP startup/teardown.
        start, frames = 48_000, 96_000
        assert len(samples) >= (start + frames) * 4, 'monitor recording too short'
        signals = [samples[4*start+c:4*(start+frames):4] for c in range(4)]
        rms = [math.sqrt(sum(v*v for v in channel)/frames) for channel in signals]
        # Main B: 880 Hz at .1/.05; cue A: 440 Hz at .2/.1 × .5.
        wanted = [0.1/math.sqrt(2), 0.05/math.sqrt(2)] * 2
        for actual, expected in zip(rms, wanted):
            assert abs(actual-expected) < expected*0.05, (rms, wanted)
        def amplitude(channel, hz):
            re = sum(v*math.cos(i*hz*math.tau/48_000) for i,v in enumerate(channel))
            im = sum(v*math.sin(i*hz*math.tau/48_000) for i,v in enumerate(channel))
            return 2*math.hypot(re, im)/frames
        for c, signal in enumerate(signals):
            wanted_hz, rejected_hz = (880,440) if c < 2 else (440,880)
            assert amplitude(signal, rejected_hz) < amplitude(signal, wanted_hz)*0.001, 'bus leakage'
        assert command('pactl', 'get-default-sink') == before, 'default changed'
        print(json.dumps({'result': 'PASS', 'channel_rms': rms,
                          'main_hz': 880, 'headphone_hz': 440, 'leakage_below_db': -60,
                          'default_unchanged': True, 'scope': 'virtual Linux PipeWire only'}))
    finally:
        if recorder is not None:
            recorder.terminate()
            recorder.wait(timeout=5)
        sp.run(['pactl', 'unload-module', module], check=True)
        assert command('pactl', 'get-default-sink') == before
        sinks = json.loads(command('pactl', '--format=json', 'list', 'sinks'))
        assert not any(s['name'] == sink for s in sinks), 'temporary sink not removed'
        print('temporary sink removed; system default unchanged')


if __name__ == '__main__':
    main()
