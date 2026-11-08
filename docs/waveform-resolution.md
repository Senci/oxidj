# Zoomed waveform resolution

New waveform analyses target 1,200 peak columns per second (rounded to whole samples per column); the 150/s format remains readable. Each peak retains the sample minima and maxima in each band, and mip levels continue to union children. Detail mesh construction stays CPU-based and bounded by visible pixel width.

**Existing lower-resolution peaks are not upgraded automatically.** Default analysis skips unchanged sources. To explicitly upgrade one track, run `oxidj-cli catalog analyze-waveform --db <catalog> --track-id <id> --refresh`. To refresh **all catalog tracks**, omit `--track-id`: `oxidj-cli catalog analyze-waveform --db <catalog> --refresh`. Refresh rereads and decodes source audio and increases derived storage; unavailable or changed sources fail without replacing existing peaks. It does not run automatically on launch or import. No owner catalog was modified for this change.

## Small synthetic measurement

Release-mode `WaveformBuilder`, 48 kHz, 10 seconds of synthetic sine and impulses, 20 consecutive analysis runs including mip construction on this machine. Stored size is the level-0 three-band min/max blob (six bytes per column), excluding SQLite overhead; mipmaps are reconstructed in memory.

| target | samples/column | blob bytes | mean analysis time |
| --- | ---: | ---: | ---: |
| 150/s | 320 | 9,000 | 4.216 ms |
| 1,200/s | 40 | 72,000 | 4.633 ms |

This single small local measurement is not a benchmark of real music, decoding, rendering FPS, or large catalogs. It indicates an 8× peak-blob cost at 48 kHz and a modest timing difference in this particular run.
