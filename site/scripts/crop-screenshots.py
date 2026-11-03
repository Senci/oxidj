"""Crop the reviewed after-W9 captures into site images.

Run from site/:  uv run --no-project --with pillow python scripts/crop-screenshots.py

Sources are the synthetic-catalog captures in docs/milestones/ui-minimal/after-w9/
(1440x900, Bot Screen, no private library data). Each crop keeps only the app
window content (no desktop bar, title bar or dock). Every crop is written as a
lossless WebP at native resolution (2x and the lightbox); the wide overview
also gets a half-width lossy WebP for 1x screens. Small panel crops are
shown near native size, so a half-size copy would only add bytes. The social card is composed from the overview crop.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

SITE = Path(__file__).resolve().parent.parent
SRC = SITE.parent / "docs/milestones/ui-minimal/after-w9"
OUT = SITE / "public/images"
MONO = SRC / "01-details-decks-mono-night-1440.png"
BAND = SRC / "02-three-band-deck-a-play-night-1440.png"

# name: (source, (left, top, right, bottom)) in source pixels.
CROPS = {
    "app-overview": (BAND, (0, 63, 1440, 856)),
    "track-list": (MONO, (176, 97, 1080, 362)),
    "decks-bar": (BAND, (0, 789, 834, 824)),
    "waveform-mono": (MONO, (1081, 357, 1440, 762)),
    "waveform-3band": (BAND, (1081, 357, 1440, 762)),
}


HALF = {"app-overview"}


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
    social()


def social() -> None:
    w, h = 1200, 630
    card = Image.new("RGB", (w, h), (16, 16, 22))
    shot = Image.open(BAND).convert("RGB").crop(CROPS["app-overview"][1])
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
