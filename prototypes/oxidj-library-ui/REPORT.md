# Provisional M3 local-library UI smoke

Status: provisional local slice only; this is not production framework admission.

## Synthetic fixture and persisted result

- Fixture: `crates/oxidj-library/tests/fixtures/tone.flac`, copied to a private Bot Screen scratch directory; no personal media used.
- Import command: `cargo run --offline -p oxidj-cli -- catalog import --dir <scratch>/media --db <scratch>/catalog.sqlite`.
- Fresh-process CLI readback reported exactly one track: ID `1`, path `<scratch>/media/fixture.flac`, SHA-256 `23ecdd8f0f5449836225d56cb384a99f766ac05aaefc38fefd2d217b45d03131`, 8,994 bytes.
- A new GUI process opened that existing catalog without invoking import or rescanning. Bot Screen screenshot showed the same row and persisted facts: 0.04 s, 44,100 Hz, 1 channel, 16 bit, Available.
- Library test asserts details query returns exactly one row with sample rate 44,100, one channel, 16-bit depth, and Available status.

## GUI interaction/error checks

- Bot Screen on display `:20` showed the live `oxidj-library-ui` process and window.
- Live click selected ID `1`; details showed `fixture.flac`, 0.040 s, 44,100 Hz, 1 channel, 16 bit, 8,994 bytes, Available, and the digest above.
- Live GUI interaction was performed on Bot Screen display `:20` with pointer/keyboard input. Search for `fixture` narrowed the list and clearing it restored all rows; clicking ID `1` selected it and the details pane showed its facts; filename ascending produced alpha, beta, fixture and descending produced fixture, beta, alpha. GUI import selected the synthetic `m3media` folder and visibly reported `Import: 2 outcomes`.
- Expected and actual import rows: pre-existing fixture ID `1` remained; newly imported `alpha.wav` was ID `2` and `beta.wav` ID `3` (no missing or duplicate rows). Catalog readback: ID 1 `fixture.flac`, 8,994 bytes, 0.040 s, 44,100 Hz, mono, 16-bit, Available, SHA-256 `23ecdd8f0f5449836225d56cb384a99f766ac05aaefc38fefd2d217b45d03131`; ID 2 `alpha.wav`, 52 bytes, 0.000090703 s, 44,100 Hz, mono, 16-bit, Available, SHA-256 `d3c8fc6137c0007f41d22a7fe97461d4cc9d1180269636897de99c1bdeb0c61a`; ID 3 `beta.wav`, 60 bytes, 0.000083333 s, 48,000 Hz, stereo, 16-bit, Available, SHA-256 `cc661faf12b03aa2fb27776dbcd3efb2f37b233538e87dada306544b3031ff91`.
- Source immutability comparison: alpha pre-import SHA-256 `d3c8fc6137c0007f41d22a7fe97461d4cc9d1180269636897de99c1bdeb0c61a` = post-import SHA-256 (same); beta pre-import SHA-256 `cc661faf12b03aa2fb27776dbcd3efb2f37b233538e87dada306544b3031ff91` = post-import SHA-256 (same).
- Temporarily moving only synthetic `alpha.wav` away and reloading changed its availability to Missing. Restoring it restored the source hash.
- Fresh GUI process reopened the same persisted catalog after the importing process exited. It showed all three rows without rescanning.
- Fresh GUI on an empty scratch catalog rendered “Library is empty. Import a folder of supported audio references.” A nonexistent-parent catalog rendered the red catalog-open error. Neither touched the persisted acceptance catalog.

## Qualification boundary

This smoke does not pass or claim the deferred 500k/60fps, native-display, screen-reader/platform, or final-framework review gates. The GUI crate is standalone under `prototypes/oxidj-library-ui`, not admitted into the root Cargo workspace or production application dependencies.
