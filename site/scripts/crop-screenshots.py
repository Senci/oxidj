"""Crop the reviewed milestone captures into site images.

Run from site/:  uv run --no-project --with pillow python scripts/crop-screenshots.py

Sources are synthetic-catalog Bot Screen captures (1440x900, no private library
data) under docs/milestones/: v12-ui-polish (W11 shell polish) and
v13-computed-beat-grid (computed BPM and beat grid, release build of 9a9b96c).
Each crop keeps only app window content (no desktop bar, title bar, dock or
pointer badge). Every crop is written as a lossless WebP at native resolution
(2x and the lightbox); the wide overview also gets a half-width lossy WebP for
1x screens. Small panel crops are shown near native size, so a half-size copy
would only add bytes. The social card is composed from the overview crop.
Superseded files are deleted so public/images holds exactly what the page uses.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

SITE = Path(__file__).resolve().parent.parent
MILESTONES = SITE.parent / "docs/milestones"
OUT = SITE / "public/images"
NIGHT = MILESTONES / "v12-ui-polish/03-after-night.png"
LOOP = MILESTONES / "v13-computed-beat-grid/01-ventoy-style-computed-grid.png"
CLICK = MILESTONES / "v13-computed-beat-grid/02-click-track-computed-grid.png"

# Waveform section of the details panel: below the details/waveform hairline
# (y 361), right of the panel edge (x 1080), above the window bottom bar.
WAVE_PANEL = (1081, 362, 1440, 800)

# name: (source, (left, top, right, bottom)) in source pixels.
CROPS = {
    "app-overview": (NIGHT, (0, 65, 1440, 856)),
    "track-list": (NIGHT, (176, 97, 1080, 362)),
    "decks-bar": (NIGHT, (0, 801, 828, 832)),
    "waveform-beat-grid": (LOOP, WAVE_PANEL),
    "waveform-click-grid": (CLICK, WAVE_PANEL),
}

HALF = {"app-overview"}
SUPERSEDED = ("waveform-mono.webp", "waveform-3band.webp")


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
    draw.text((56, 470), "Local DJ library in Rust.", font=body, fill=(196, 190, 226))
    draw.rectangle((56, 132, 120, 138), fill=(187, 154, 247))
    card.save(OUT / "social.png", optimize=True)
    print(f"public/images/social.png {w}x{h}")


if __name__ == "__main__":
    main()
