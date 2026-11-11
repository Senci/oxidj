"""Crop the reviewed milestone captures into site images.

Run from site/:  uv run --no-project --with pillow python scripts/crop-screenshots.py

Sources are synthetic-catalog Bot Screen captures (1440x900, no private library
data) under docs/milestones/: v13-computed-beat-grid (computed BPM and beat grid,
release build of 9a9b96c), v14-merged-decks (M8 D1, release build of ef5c601),
v15-tempo-keylock-sync (M8 D3, release build of fc46d0f) and
v17-waveform-bands (W12 3-band deck waveforms, release build of d1e6c21; its
01 is a copy of the v16 mixer capture and serves as the "before"), and
v18-imported-playlists (approved imported-playlist UI, installed release of
8068744).
Each crop keeps only app window content (no desktop bar, title bar, dock or
pointer badge). Every crop is written as a lossless WebP at native resolution
(2x and the lightbox); full-width crops also get a half-width lossy WebP for
1x screens. Small crops are shown near native size, so a half-size copy would
only add bytes. The social card is composed from the overview crop.
Superseded files are deleted so public/images holds exactly what the page uses.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

SITE = Path(__file__).resolve().parent.parent
MILESTONES = SITE.parent / "docs/milestones"
OUT = SITE / "public/images"
LOOP = MILESTONES / "v13-computed-beat-grid/01-ventoy-style-computed-grid.png"
CLICK = MILESTONES / "v13-computed-beat-grid/02-click-track-computed-grid.png"
FALLBACK = MILESTONES / "v14-merged-decks/04-grid-fallback-no-grid-and-analysing.png"
COLLAPSED = MILESTONES / "v14-merged-decks/05-collapsed-detail-library-grows.png"
REFUSED = MILESTONES / "v15-tempo-keylock-sync/04-sync-refused-no-beat-grid.png"
BANDS = MILESTONES / "v17-waveform-bands"
BEFORE = BANDS / "01-before-v16-monochrome.png"
PLAYING = BANDS / "03-after-both-playing-played-dimmed.png"
SYNCED = BANDS / "04-after-sync-keylock-both-playing.png"
PLAYLISTS = MILESTONES / "v18-imported-playlists/01-imported-playlist-load-controls.png"
LAYOUTS = MILESTONES / "v19-deck-layout-v2"

# Waveform section of the details panel: below the details/waveform hairline
# (y 361), right of the panel edge (x 1080), above the window bottom bar.
WAVE_PANEL = (1081, 362, 1440, 800)
# v14 deck area: below the toolbar (y 97) through the two overviews (end y 486).
DECK_AREA = (0, 100, 1440, 492)
# v16/v17 deck area with the mixer: merged detail waveform (from y 130), control
# strip, mixer and both overviews (end y 550); the pointer sits below y 575.
BANDS_AREA = (0, 130, 1440, 552)

# name: (source, (left, top, right, bottom)) in source pixels.
CROPS = {
    # Latest layout B, stopping before the table and Details panel.
    "app-overview": (LAYOUTS / "app-deck-row-fit-1440x790.png", (0, 0, 1440, 635)),
    "deck-mixer-v2": (LAYOUTS / "app-deck-row-fit-1440x790.png", (0, 350, 1440, 635)),
    # Synthetic v19 application screenshots, cropped above the Details path.
    "layout-a": (LAYOUTS / "pages-layout-a-synthetic.png", (0, 0, 1440, 628)),
    # Newer 1440x790 synthetic capture: complete deck row, no table or Details.
    "layout-b": (LAYOUTS / "app-deck-row-fit-1440x790.png", (0, 0, 1440, 635)),
    "layout-c": (LAYOUTS / "pages-layout-c-synthetic.png", (0, 0, 1440, 628)),
    "layout-d": (LAYOUTS / "pages-layout-d-synthetic.png", (0, 0, 1440, 473)),
    # DECKS row and the merged 3-band detail waveform, both decks playing.
    "decks-merged": (PLAYING, (0, 100, 1440, 356)),
    # Seam readout, both tempo faders and KEY/SYNC lit on deck B.
    "decks-synced": (SYNCED, (440, 128, 1000, 384)),
    # Deck B's half of the control strip with the visible SYNC refusal.
    "sync-refused": (REFUSED, (720, 356, 1440, 412)),
    # Control strip and the mixer row with live meters (overview A at y 478).
    "decks-mixer": (SYNCED, (0, 356, 1440, 476)),
    # Same deck area before (v16 near-monochrome) and after W12 (3 bands).
    "bands-before": (BEFORE, BANDS_AREA),
    "bands-after": (PLAYING, BANDS_AREA),
    "grid-fallback": (FALLBACK, DECK_AREA),
    # Detail collapsed: strip and overviews, then the track table grows.
    "decks-collapsed": (COLLAPSED, (0, 100, 1440, 400)),
    "waveform-beat-grid": (LOOP, WAVE_PANEL),
    "waveform-click-grid": (CLICK, WAVE_PANEL),
    # Actual imported-playlist panel, selected member and explicit Load A/B.
    "imported-playlist-load-controls": (PLAYLISTS, (0, 0, 175, 252)),
}

HALF = {
    "app-overview",
    "deck-mixer-v2",
    "layout-a",
    "layout-b",
    "layout-c",
    "layout-d",
    "decks-merged",
    "decks-mixer",
    "bands-before",
    "bands-after",
    "grid-fallback",
    "decks-collapsed",
}
SUPERSEDED = (
    "waveform-mono.webp",
    "waveform-3band.webp",
    "track-list.webp",
    "decks-bar.webp",
)


def save(img: Image.Image, path: Path, lossless: bool = True) -> None:
    if lossless:
        img.save(path, "WEBP", lossless=True, method=6)
    else:
        img.save(path, "WEBP", quality=80, method=6)
    print(f"{path.relative_to(SITE)} {img.width}x{img.height} {path.stat().st_size} B")


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (src, box) in CROPS.items():
        img = Image.open(src).convert("RGB").crop(box)
        save(img, OUT / f"{name}.webp")
        if name in HALF:
            half = img.resize((img.width // 2, img.height // 2), Image.Resampling.LANCZOS)
            save(half, OUT / f"{name}-{half.width}.webp", lossless=False)
    for stale in SUPERSEDED:
        (OUT / stale).unlink(missing_ok=True)
    social()


def social() -> None:
    w, h = 1200, 630
    card = Image.new("RGB", (w, h), (16, 16, 22))
    src, box = CROPS["app-overview"]
    shot = Image.open(src).convert("RGB").crop(box)
    shot = shot.resize((760, round(760 * shot.height / shot.width)), Image.Resampling.LANCZOS)
    card.paste(shot, (w - shot.width + 120, 150))
    draw = ImageDraw.Draw(card)
    bold = ImageFont.truetype(str(SITE / "fonts-src/epilogue-700.ttf"), 78)
    body = ImageFont.truetype(str(SITE / "fonts-src/epilogue-400.ttf"), 30)
    draw.rectangle((0, 0, 470, h), fill=(16, 16, 22))
    draw.text((56, 170), "Your music.\nYour decks.", font=bold, fill=(234, 232, 247), spacing=6)
    draw.text((56, 470), "Library and two decks, in Rust.", font=body, fill=(196, 190, 226))
    draw.rectangle((56, 132, 120, 138), fill=(187, 154, 247))
    card.save(OUT / "social.png", optimize=True)
    print(f"public/images/social.png {w}x{h}")


if __name__ == "__main__":
    main()
