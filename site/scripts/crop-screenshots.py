"""Crop the reviewed milestone captures into site images.

Run from site/:  uv run --no-project --with pillow python scripts/crop-screenshots.py

Sources are synthetic-catalog Bot Screen captures (1440x900, no private library
data) under docs/milestones/: v13-computed-beat-grid (computed BPM and beat grid,
release build of 9a9b96c), v14-merged-decks (M8 D1, release build of ef5c601),
v15-tempo-keylock-sync (M8 D3, release build of fc46d0f) and v16-mixer (M8 D4).
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
LOADED = MILESTONES / "v14-merged-decks/02-after-both-decks-loaded.png"
FALLBACK = MILESTONES / "v14-merged-decks/04-grid-fallback-no-grid-and-analysing.png"
COLLAPSED = MILESTONES / "v14-merged-decks/05-collapsed-detail-library-grows.png"
SYNCED = MILESTONES / "v15-tempo-keylock-sync/02-after-sync-b-keylock-matched.png"
REFUSED = MILESTONES / "v15-tempo-keylock-sync/04-sync-refused-no-beat-grid.png"
MIXER = MILESTONES / "v16-mixer/01-mixer-both-playing-centre.png"

# Waveform section of the details panel: below the details/waveform hairline
# (y 361), right of the panel edge (x 1080), above the window bottom bar.
WAVE_PANEL = (1081, 362, 1440, 800)
# Deck area: below the toolbar (y 97) through the two overviews (end y 486).
DECK_AREA = (0, 100, 1440, 492)

# name: (source, (left, top, right, bottom)) in source pixels.
CROPS = {
    # Toolbar, merged decks, control strip, mixer, overviews and the four
    # table rows; stops above the pointer badge in the empty table (y 760).
    "app-overview": (MIXER, (0, 65, 1440, 745)),
    "decks-merged": (LOADED, DECK_AREA),
    # Seam readout, both tempo faders and KEY/SYNC lit on deck B.
    "decks-synced": (SYNCED, (440, 128, 1000, 384)),
    # Deck B's half of the control strip with the visible SYNC refusal.
    "sync-refused": (REFUSED, (720, 356, 1440, 412)),
    # Control strip and the mixer row (overview A starts at y 478).
    "decks-mixer": (MIXER, (0, 356, 1440, 476)),
    "grid-fallback": (FALLBACK, DECK_AREA),
    # Detail collapsed: strip and overviews, then the track table grows.
    "decks-collapsed": (COLLAPSED, (0, 100, 1440, 400)),
    "waveform-beat-grid": (LOOP, WAVE_PANEL),
    "waveform-click-grid": (CLICK, WAVE_PANEL),
}

HALF = {"app-overview", "decks-merged", "decks-mixer", "grid-fallback", "decks-collapsed"}
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
        img.save(path, "WEBP", quality=84, method=6)
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
    draw.text((56, 170), "A library\nbefore\nthe decks.", font=bold, fill=(234, 232, 247), spacing=6)
    draw.text((56, 470), "Library and two decks, in Rust.", font=body, fill=(196, 190, 226))
    draw.rectangle((56, 132, 120, 138), fill=(187, 154, 247))
    card.save(OUT / "social.png", optimize=True)
    print(f"public/images/social.png {w}x{h}")


if __name__ == "__main__":
    main()
