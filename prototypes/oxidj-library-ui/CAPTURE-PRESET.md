# Experimental silent capture preset

This opt-in prototype prepares two decks with the no-device silent engine. It
is **not a screenshot tool**. Use a disposable copy of a catalog and an
independent, supported Bot Screen capture. A copied catalog may acquire empty
SQLite WAL/SHM sidecars even when opened read-only; the source catalog and audio
must remain unchanged. Capture-mode artwork is loaded read-only without updating
the persistent artwork cache.

The preset takes explicit track IDs, track-time positions and mixer/display
settings in JSON. It requires an absolute disposable catalog path and the
explicit `--capture-disposable-db` acknowledgement. It loads both decks,
acknowledges seeks and SYNC while paused, then exposes a one-shot START button.
The optional `tempo_range` is 6, 10 or 16 percent; omitting it selects 10.
For example, with **synthetic** track IDs and positions:

```json
{
  "track_ids": [1, 2],
  "positions_ms": [4000, 5000],
  "crossfader": 0.5,
  "master": 1.0,
  "sync_deck": 1,
  "tempo_range": 16,
  "faders": [1.0, 1.0],
  "filters": [0.0, 0.0],
  "eq_db": [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
  "palette": "classic",
  "sections": [true, true, true, true]
}
```

`sync_deck` uses zero-based deck indices; the two filter values range from
-1 to 1, and each EQ array is ordered low, mid, high in dB. Sections are
detail, overviews, strip, mixer. Launch with:

```sh
oxidj-library-ui --db /absolute/path/to/disposable.sqlite \
  --capture-preset /absolute/path/to/private-preset.json \
  --capture-disposable-db
```

SYNC can adjust the follower's phase: displayed seconds from an earlier
screenshot do not identify a unique alignment. A fresh screenshot, actual
owner-approved alignment and exact timing are not demonstrated by these tests.

Normal startup continues to use the configured audio device; capture startup
uses `Engine::silent`, opens no audio device, and does not save settings. The
NO AUDIO status is intentional and must not be misrepresented as hardware
playback. This prototype requires review and live verification before use.
