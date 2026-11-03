"""WCAG contrast of the OxiDJ library-UI token pairs (W7 design audit)."""
import sys

N = dict(background=0x1a1b26, surface=0x242536, text=0xc0caf5, muted=0xa9b1d6, primary=0xbb9af7,
         secondary=0x7dcfff, border=0x3b3d55, focus=0x7dcfff, success=0x9ece6a, warning=0xe0af68, error=0xf7768e)
D = dict(background=0xe1e2e7, surface=0xf4f4f5, text=0x2f50a0, muted=0x556485, primary=0x9854f1,
         secondary=0x005a78, border=0xb8bed0, focus=0x007197, success=0x587539, warning=0x8c6c3d, error=0xc64343)
if len(sys.argv) > 1:  # proposed overrides: name=hex,...
    for kv in sys.argv[1].split(","):
        theme, k, v = kv.split(":")
        (N if theme == "n" else D)[k] = int(v, 16)

rgb = lambda c: ((c >> 16) & 255, (c >> 8) & 255, c & 255)
def lum(c):
    f = lambda v: v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
    r, g, b = (f(x / 255) for x in rgb(c))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b
def ratio(a, b):
    la, lb = sorted((lum(a), lum(b)), reverse=True)
    return (la + 0.05) / (lb + 0.05)
def over(fg, a, bg):  # egui linear_multiply(a) premultiplied over bg (approx in sRGB)
    return tuple(round(f * a + b * (1 - a)) for f, b in zip(rgb(fg), rgb(bg)))
pack = lambda t: (t[0] << 16) | (t[1] << 8) | t[2]

for name, t in (("Tokyo Night", N), ("Tokyo Day", D)):
    print(f"## {name}")
    rows = []
    for fg in ("text", "muted", "secondary", "primary", "success", "warning", "error"):
        for bg in ("background", "surface"):
            rows.append((f"{fg} on {bg}", ratio(t[fg], t[bg]), 4.5 if fg in ("text", "muted", "secondary") else 3.0))
    for k in ("success", "warning", "error"):
        fill = pack(over(t[k], 0.16, t["surface"]))
        rows.append((f"text on {k} badge (surface)", ratio(t["text"], fill), 4.5))
        fill = pack(over(t[k], 0.16, t["background"]))
        rows.append((f"text on {k} badge (background)", ratio(t["text"], fill), 4.5))
    sel = pack(over(t["primary"], 0.20, t["background"]))  # theme::SELECTION_TINT
    rows.append(("text on selection (background)", ratio(t["text"], sel), 4.5))
    act = pack(over(t["primary"], 0.18, t["background"]))
    rows.append(("text on primary action", ratio(t["text"], act), 4.5))
    rows.append(("border on background (non-text)", ratio(t["border"], t["background"]), 3.0))
    rows.append(("border on surface (non-text)", ratio(t["border"], t["surface"]), 3.0))
    for label, r, need in rows:
        print(f"{'PASS' if r >= need else 'FAIL'}  {r:5.2f}:1 (>= {need})  {label}")
