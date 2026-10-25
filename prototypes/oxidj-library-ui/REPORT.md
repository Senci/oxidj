# Provisional M3 local-library UI smoke

Status: provisional local slice only; this is not production framework admission.

## Synthetic fixture and persisted result

- Fixture: `crates/oxidj-library/tests/fixtures/tone.flac`, copied to a private Bot Screen scratch directory; no personal media used.
- Import command: `cargo run --offline -p oxidj-cli -- catalog import --dir <scratch>/media --db <scratch>/catalog.sqlite`.
- Fresh-process CLI readback reported exactly one track: ID `1`, path `<scratch>/media/fixture.flac`, SHA-256 `23ecdd8f0f5449836225d56cb384a99f766ac05aaefc38fefd2d217b45d03131`, 8,994 bytes.
- A new GUI process opened that existing catalog without invoking import or rescanning. Bot Screen screenshot showed the same row and persisted facts: 0.04 s, 44,100 Hz, 1 channel, 16 bit, Available.
- Library test asserts details query returns exactly one row with sample rate 44,100, one channel, 16-bit depth, and Available status.

## GUI interaction/error checks

- Bot Screen is isolated at display `:20`; GUI process/window was observed live as `oxidj-library-ui` / `OxiDJ Local Library`.
- A coordinate click through the generic desktop input route was unverified and did not change app state. After calibrating exact X11 window bounds from the live screenshot, an XTest press/release at the filename activated the row; a fresh Bot Screen capture showed ID `1` selected and the details pane populated with filename/path, 0.040 s, 44,100 Hz, 1 channel, 16 bit, 8,994 bytes, Available, and the exact digest above. See `evidence/selected-track-details.png`.
- The first GUI launch without the Bot Screen environment failed with Winit's expected missing-display error; relaunch with the private environment succeeded. This is environment setup.
- A separate GUI process pointed at a database path whose parent directory does not exist and rendered the catalog-open error in red; see `evidence/catalog-error.png`. This did not modify the valid fixture catalog.
- Search, sort, empty, and missing-file states are implemented but were not all live-interacted in this smoke. Missing availability is also determined by current path existence on query.

## Qualification boundary

This smoke does not pass or claim the deferred 500k/60fps, native-display, screen-reader/platform, or final-framework review gates. The GUI crate is standalone under `prototypes/oxidj-library-ui`, not admitted into the root Cargo workspace or production application dependencies.
