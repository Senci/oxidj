"""W6 captures: add a synthetic imported-library beat grid to a scratch catalog.

Run after `catalog init/import/analyze-waveform` on the gen_synthetic.py audio
mounted at /mnt (see README). Inserts two library tracks keyed by the synthetic
files' locations: the 124 bpm multiband track gets a 4/4 grid starting at 0 ms
(beat_in_bar 1..4), the hats-only track gets none (8 s fallback). Synthetic only.
"""
import sqlite3, sys

c = sqlite3.connect(sys.argv[1])
c.execute("INSERT INTO library_tracks(position,title,tempo_hundredths_bpm,duration_seconds,location)"
          " VALUES (0,'Synthetic 124 bpm',12400,48,'file://localhost/mnt/audio/synthetic-multiband.wav')")
c.execute("INSERT INTO library_tracks(position,title,tempo_hundredths_bpm,duration_seconds,location)"
          " VALUES (1,'Synthetic hats (no grid)',12400,12,'file://localhost/mnt/audio/synthetic-hats-only.wav')")
beat, n = 60000 / 124, 0
while round(n * beat) < 48000:
    c.execute("INSERT INTO library_beats VALUES (0,?,?,12400,?)", (n, round(n * beat), n % 4 + 1))
    n += 1
c.commit()
print("beats", n)
