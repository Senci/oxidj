"""Synthetic multi-band test audio for the W4 waveform captures (no real music).

synthetic-multiband.wav (48 s, 44.1 kHz, 16-bit mono):
  low  - 55 Hz decaying kick on every beat at 124 bpm, absent in bars 9-12 (breakdown)
  mid  - 440/554/659 Hz triad pad, amplitude rising over 16 bars
  high - 8 kHz-ish noise hats on off-beats, plus a 16th-note roll in bars 13-16
synthetic-hats-only.wav (12 s): high band only (for a contrasting track).
"""
import math, random, struct, sys, wave

RATE = 44100
random.seed(7)

def write(path, samples):
    with wave.open(path, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(RATE)
        w.writeframes(b"".join(struct.pack("<h", int(max(-1, min(1, s)) * 32000)) for s in samples))

def hp_noise(n):
    # first-difference of white noise: strongly high-frequency weighted
    prev, out = 0.0, []
    for _ in range(n):
        x = random.uniform(-1, 1); out.append(x - prev); prev = x
    return out

def multiband(seconds=48.0):
    n = int(seconds * RATE); beat = 60 / 124; noise = hp_noise(n); out = []
    for i in range(n):
        t = i / RATE; b = t / beat; bar = int(b // 4); ph = b % 1
        kick = 0.0 if 8 <= bar < 12 else math.sin(math.tau * 55 * t) * math.exp(-ph * beat * 9)
        pad = (0.08 + 0.22 * min(1, t / (16 * 4 * beat))) * sum(
            math.sin(math.tau * f * t) for f in (440, 554.37, 659.25)) / 3
        hat_env = math.exp(-((b + 0.5) % 1) * beat * 40)
        if 12 <= bar < 16:
            hat_env = max(hat_env, math.exp(-((b * 4) % 1) * beat / 4 * 60))
        out.append(0.62 * kick + pad + 0.22 * hat_env * noise[i])
    return out

def hats_only(seconds=12.0):
    n = int(seconds * RATE); beat = 60 / 124; noise = hp_noise(n)
    return [0.3 * math.exp(-((i / RATE / beat + 0.5) % 1) * beat * 30) * noise[i] for i in range(n)]

if __name__ == "__main__":
    d = sys.argv[1]
    write(f"{d}/synthetic-multiband.wav", multiband())
    write(f"{d}/synthetic-hats-only.wav", hats_only())
