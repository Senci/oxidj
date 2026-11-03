"""Subset the self-hosted fonts to Latin and compress them to WOFF2.

Run from site/:  uv run --no-project --with fonttools --with brotli python scripts/subset-fonts.py

Sources are the original TTFs in site/fonts-src/ (not served). Both fonts are under
the SIL Open Font License 1.1; the licence texts ship in public/fonts/. Subsetting
makes a Modified Version. IBM Plex declares the Reserved Font Name "Plex", so the
Plex Mono subset is renamed "OxiDJ Mono" as the licence requires; Epilogue has no
Reserved Font Name and keeps its name.
"""

from pathlib import Path

from fontTools import subset

SITE = Path(__file__).resolve().parent.parent
SRC = SITE / "fonts-src"
OUT = SITE / "public/fonts"
# Basic Latin, Latin-1 Supplement and common punctuation.
UNICODES = "U+0020-007E,U+00A0-00FF,U+2010-2027,U+2030-203A,U+20AC,U+2122"
RENAME = {"ibm-plex-mono-400": ("OxiDJ Mono", "oxidj-mono-400")}


def rename(font, family: str) -> None:
    names = {1: family, 16: family, 4: f"{family} Regular", 3: f"{family} Regular; web subset",
             6: family.replace(" ", "") + "-Regular", 17: "Regular"}
    for record in font["name"].names:
        if record.nameID in names:
            record.string = names[record.nameID]


for ttf in sorted(SRC.glob("*.ttf")):
    family, stem = RENAME.get(ttf.stem, (None, ttf.stem))
    target = OUT / f"{stem}.woff2"
    options = subset.Options()
    options.flavor = "woff2"
    options.layout_features = ["kern", "liga", "calt", "tnum"]
    options.name_IDs = ["*"]
    font = subset.load_font(str(ttf), options)
    subsetter = subset.Subsetter(options)
    subsetter.populate(unicodes=subset.parse_unicodes(UNICODES))
    subsetter.subset(font)
    if family:
        rename(font, family)
    subset.save_font(font, str(target), options)
    print(f"{target.relative_to(SITE)} {ttf.stat().st_size} -> {target.stat().st_size} B")
