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

## Full-width playlist and track interaction follow-on

Imported playlist selection opens its ordered members (including duplicates) in
one full-width, virtualized table: Title, Artist, Album, BPM, Key and Duration.
The sidebar contains navigation only. Search matches title, artist and album;
playlist order is preserved rather than silently re-sorted. Selecting a member
shows its imported metadata in Details, including imported-only tracks without a
scanned audio reference. All tracks returns to the sortable collection table.
The cached lightweight navigation query includes table metadata, never waveform
blobs; a normal frame does not re-read the whole imported snapshot.

Drag a collection or playlist row onto either deck's detail waveform, overview
or control-strip half. The target is outlined; release loads that deck stopped.
Esc, release outside a deck, or hiding the decks cancels without loading.
Imported audio resolves by the existing exact-location path; unavailable audio
is refused without replacing the loaded deck. Context-menu and Load-button
paths remain available.

Settings → Keyboard offers an opt-in **Simple DJ keys (Rekordbox-inspired)**
mode, persisted with the other settings. It is a small mapping for supported
actions, not an exact Rekordbox shortcut compatibility promise:

| Keys | Action |
|---|---|
| A / B | Load selected collection or playlist track onto deck A / B |
| Z / X | Cue (hold/release) / play-pause deck A in Simple mode |
| N / M | Cue (hold/release) / play-pause deck B in Simple mode |
| Q / P | Cue deck A / B in the default, standard mode |
| Space, Tab, 1–8 | Existing active-deck play, switch, and hot-cue controls |
| Esc | Cancel an active track drag |

DJ shortcuts do not steal text entry or focused-control keys and are inactive
while Settings is open or decks are hidden. Moving focus away releases a held
cue preview. Existing panel/theme chords are unchanged.

Synthetic production-egui coverage is in `library_interaction_tests.rs` and
`shell_tests.rs`, with settings persistence in `settings.rs`. It exercises real
press/move/release frames, both row sources and decks, no-autoplay/cancellation,
missing audio, metadata/details/search, scroll-to-last-member, and guarded
transport keys. This is headless egui interaction evidence, not a fresh live
Bot Screen/native input or accessibility qualification claim.

## Qualification boundary

This smoke does not pass or claim the deferred 500k/60fps, native-display, screen-reader/platform, or final-framework review gates. The GUI crate is standalone under `prototypes/oxidj-library-ui`, not admitted into the root Cargo workspace or production application dependencies.
