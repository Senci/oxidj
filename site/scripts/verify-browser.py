"""Rendered-site audit with Playwright driving the system Chromium.

    uv run --no-project --with playwright python scripts/verify-browser.py [base-url] [out-dir]

Serve the build first, e.g. `dist/` mounted at /oxidj/ on 127.0.0.1:4190.
Checks every route at 390 and 1440 px: console errors, failed requests,
horizontal overflow, tap targets < 44 px (mobile), WCAG contrast of visible
text against its effective background, layout shift, transferred bytes, and
the lightbox (open, wheel zoom, drag, keyboard, Esc, outside click, real
mouse click/double-click on the image, touch taps (hasTouch context), focus
return). Writes full-page reduced-motion captures and viewport captures with
motion to the output directory and prints a JSON summary. Exit 1 on failure.
"""

import json
import os
import sys
from pathlib import Path

from playwright.sync_api import sync_playwright

BASE = sys.argv[1] if len(sys.argv) > 1 else "http://127.0.0.1:4190/oxidj/"
OUT = Path(sys.argv[2] if len(sys.argv) > 2 else ".hallmark/final")
CHROME = os.environ.get("CHROME_PATH", "/usr/sbin/chromium")
ROUTES = {"index": "", "roadmap": "roadmap.html", "dossier": "dossier.html", "404": "404.html"}

AUDIT_JS = r"""
() => {
  const ctx = document.createElement('canvas').getContext('2d', { willReadFrequently: true });
  const rgba = (c) => { ctx.clearRect(0,0,1,1); ctx.fillStyle = '#000'; ctx.fillStyle = c; ctx.fillRect(0,0,1,1); return [...ctx.getImageData(0,0,1,1).data]; };
  const lum = (rgb) => { const c = rgb.slice(0,3).map(v => { v/=255; return v <= 0.04045 ? v/12.92 : ((v+0.055)/1.055)**2.4; }); return 0.2126*c[0]+0.7152*c[1]+0.0722*c[2]; };
  const blend = (fg, bg) => { const a = fg[3]/255; return [0,1,2].map(i => fg[i]*a + bg[i]*(1-a)).concat(255); };
  const bgOf = (el) => {
    const stack = [];
    for (let n = el; n; n = n.parentElement) { const c = rgba(getComputedStyle(n).backgroundColor); if (c[3] > 0) stack.push(c); if (c[3] === 255) break; }
    let bg = rgba(getComputedStyle(document.documentElement).backgroundColor);
    for (const c of stack.reverse()) bg = blend(c, bg);
    return bg;
  };
  const visible = (el) => { const r = el.getBoundingClientRect(); const s = getComputedStyle(el); return r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none' && !el.closest('[hidden],[aria-hidden="true"],.visually-hidden'); };
  const textEls = [...document.querySelectorAll('h1,h2,h3,p,li,dt,dd,a,button,summary,figcaption,time,span,code,strong')].filter(el => visible(el) && [...el.childNodes].some(n => n.nodeType === 3 && n.textContent.trim()));
  const low = [];
  let minRatio = 99;
  for (const el of textEls) {
    const s = getComputedStyle(el);
    const bg = bgOf(el);
    const fg = blend(rgba(s.color), bg);
    const [a, b] = [lum(fg), lum(bg)].sort((x, y) => x - y);
    const ratio = (b + 0.05) / (a + 0.05);
    minRatio = Math.min(minRatio, ratio);
    if (ratio < 4.5) low.push({ text: el.textContent.trim().slice(0, 40), ratio: +ratio.toFixed(2), color: s.color });
  }
  const targets = [...document.querySelectorAll('a,button,summary,[tabindex="0"]')].filter(visible).map(el => { const r = el.getBoundingClientRect(); return { text: (el.textContent || el.getAttribute('aria-label') || '').trim().slice(0, 30), w: Math.round(r.width), h: Math.round(r.height), inline: !!el.closest('p') }; }).filter(t => !t.inline && (t.w < 44 || t.h < 44));
  const imgs = [...document.images].filter(i => !i.closest('.lightbox')).map(i => ({ src: i.currentSrc.split('/').pop(), ok: i.complete && i.naturalWidth > 0, hasSize: i.hasAttribute('width') && i.hasAttribute('height'), alt: i.alt.length }));
  return {
    overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    minContrast: +minRatio.toFixed(2), lowContrast: low.slice(0, 10), smallTargets: targets.slice(0, 10), images: imgs,
    fonts: [...document.fonts].filter(f => f.status === 'loaded').map(f => f.family + ' ' + f.weight),
    bytes: performance.getEntriesByType('resource').reduce((s, e) => s + (e.transferSize || e.encodedBodySize || 0), 0) + (performance.getEntriesByType('navigation')[0]?.transferSize || 0),
    cls: window.__cls || 0,
  };
}
"""
CLS_INIT = "window.__cls = 0; new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) window.__cls += e.value; }).observe({ type: 'layout-shift', buffered: true });"


SCALE_JS = "new DOMMatrix(getComputedStyle(document.querySelector('.lightbox-image')).transform).a"


def open_and_centre(page, trigger, dialog):
    trigger.click()
    dialog.wait_for(state="visible")
    page.wait_for_timeout(450)
    box = page.locator(".lightbox-image").bounding_box()
    return box["x"] + box["width"] / 2, box["y"] + box["height"] / 2


def image_click_check(page, trigger, dialog):
    """Real mouse clicks on the image: a click keeps the dialog open, a double-click zooms."""
    r = {}
    cx, cy = open_and_centre(page, trigger, dialog)
    page.mouse.click(cx, cy)
    page.wait_for_timeout(350)  # past the double-click interval
    r["clickImageStaysOpen"] = dialog.is_visible()
    page.mouse.dblclick(cx, cy)
    page.wait_for_timeout(100)
    r["dblclickZooms"] = dialog.is_visible() and page.evaluate(SCALE_JS) > 1.5
    page.mouse.dblclick(cx, cy)
    page.wait_for_timeout(100)
    r["dblclickAgainResets"] = dialog.is_visible() and abs(page.evaluate(SCALE_JS) - 1) < 1e-3
    page.mouse.move(cx, cy)
    page.mouse.down()
    page.mouse.move(cx + 60, cy + 20, steps=6)
    page.mouse.up()
    r["dragOnImageStaysOpen"] = dialog.is_visible()
    vw = page.viewport_size
    page.mouse.click(8, vw["height"] - 8)
    r["scrimClickClosesAfter"] = not dialog.is_visible()
    return r


def touch_check(browser, failures):
    """Real touch taps (hasTouch context): tap on the image keeps it open, tap on the scrim closes."""
    ctx = browser.new_context(viewport={"width": 390, "height": 844}, has_touch=True, is_mobile=True, reduced_motion="reduce")
    page = ctx.new_page()
    page.goto(BASE, wait_until="networkidle")
    trigger = page.locator("a[data-lightbox]").first
    trigger.scroll_into_view_if_needed()
    trigger.tap()
    dialog = page.locator(".lightbox")
    dialog.wait_for(state="visible")
    page.wait_for_timeout(450)
    box = page.locator(".lightbox-image").bounding_box()
    page.touchscreen.tap(box["x"] + box["width"] / 2, box["y"] + box["height"] / 2)
    page.wait_for_timeout(350)
    r = {"tapImageStaysOpen": dialog.is_visible()}
    page.touchscreen.tap(8, 836)
    page.wait_for_timeout(100)
    r["tapScrimCloses"] = not dialog.is_visible()
    ctx.close()
    if not all(r.values()):
        failures.append(f"lightbox touch 390: {r}")
    return r


def every_trigger_check(page, failures):
    """Each lightbox link opens its own full image, zooms, pans, closes on Esc and returns focus to itself."""
    dialog = page.locator(".lightbox")
    vw = page.viewport_size
    results = []
    for i in range(page.locator("a[data-lightbox]").count()):
        trigger = page.locator("a[data-lightbox]").nth(i)
        trigger.scroll_into_view_if_needed()
        trigger.focus()
        page.keyboard.press("Enter")
        dialog.wait_for(state="visible")
        page.wait_for_timeout(450)
        href = trigger.get_attribute("href").lstrip("./")
        r = {"href": href}
        r["src"] = page.evaluate("document.querySelector('.lightbox-image').getAttribute('src')").endswith(href)
        r["natural"] = page.evaluate("document.querySelector('.lightbox-image').naturalWidth") == int(trigger.get_attribute("data-width"))
        page.mouse.move(vw["width"] / 2, vw["height"] / 2)
        for _ in range(4):
            page.mouse.wheel(0, -120)
        page.wait_for_timeout(100)
        r["zoom"] = page.evaluate(SCALE_JS) > 1.2
        # Pan on either axis: a tall, narrow crop zoomed at 1440 px may still fit horizontally.
        shift = "(() => { const m = new DOMMatrix(getComputedStyle(document.querySelector('.lightbox-image')).transform); return [m.e, m.f]; })()"
        x0, y0 = page.evaluate(shift)
        page.mouse.down()
        page.mouse.move(vw["width"] / 2 - 60, vw["height"] / 2 - 60, steps=6)
        page.mouse.up()
        x1, y1 = page.evaluate(shift)
        r["pan"] = max(abs(x1 - x0), abs(y1 - y0)) > 10
        page.keyboard.press("Escape")
        r["closed"] = not dialog.is_visible()
        r["focusReturned"] = page.evaluate(f"document.activeElement === document.querySelectorAll('a[data-lightbox]')[{i}]")
        results.append(r)
        if not all(v for k, v in r.items() if k != "href"):
            failures.append(f"lightbox trigger {vw['width']}: {r}")
    return results


def lightbox_check(page, failures):
    page.evaluate("window.scrollTo(0, 0)")
    trigger = page.locator("a[data-lightbox]").first
    trigger.scroll_into_view_if_needed()
    trigger.focus()
    page.keyboard.press("Enter")
    dialog = page.locator(".lightbox")
    dialog.wait_for(state="visible")
    page.wait_for_timeout(450)
    result = {
        "role": dialog.get_attribute("role"),
        "modal": dialog.get_attribute("aria-modal"),
        "focusClose": page.evaluate("document.activeElement?.classList.contains('lightbox-close')"),
        "opacity": page.evaluate("getComputedStyle(document.querySelector('.lightbox')).opacity"),
    }
    box = page.locator(".lightbox-image").bounding_box()
    vw = page.viewport_size
    page.screenshot(path=str(OUT / f"index-{vw['width']}-lightbox.png"))
    result["imageFits"] = bool(box and box["width"] <= vw["width"] and box["height"] <= vw["height"])
    page.mouse.move(vw["width"] / 2 + 80, vw["height"] / 2)
    for _ in range(6):
        page.mouse.wheel(0, -120)
    page.wait_for_timeout(100)
    page.screenshot(path=str(OUT / f"index-{vw['width']}-lightbox-zoomed.png"))
    zoomed = page.evaluate("getComputedStyle(document.querySelector('.lightbox-image')).transform")
    result["wheelZoom"] = zoomed
    x0 = page.evaluate("new DOMMatrix(getComputedStyle(document.querySelector('.lightbox-image')).transform).e")
    page.mouse.move(vw["width"] / 2, vw["height"] / 2)
    page.mouse.down()
    page.mouse.move(vw["width"] / 2 - 120, vw["height"] / 2 - 40, steps=8)
    page.mouse.up()
    x1 = page.evaluate("new DOMMatrix(getComputedStyle(document.querySelector('.lightbox-image')).transform).e")
    result["dragPanned"] = round(x1 - x0, 1)
    result["stillOpenAfterDrag"] = dialog.is_visible()
    page.keyboard.press("0")
    result["resetScale"] = page.evaluate("new DOMMatrix(getComputedStyle(document.querySelector('.lightbox-image')).transform).a")
    page.keyboard.press("Tab")
    page.keyboard.press("Tab")
    result["focusTrapped"] = page.evaluate("!!document.activeElement?.closest('.lightbox')")
    page.keyboard.press("Escape")
    result["escCloses"] = not dialog.is_visible()
    result["focusReturned"] = page.evaluate("!!document.activeElement?.matches('a[data-lightbox]')")
    trigger.click()
    dialog.wait_for(state="visible")
    page.wait_for_timeout(300)
    page.mouse.click(8, vw["height"] - 8)
    result["outsideClickCloses"] = not dialog.is_visible()
    result.update(image_click_check(page, trigger, dialog))
    result["everyTrigger"] = every_trigger_check(page, failures)
    ok = (
        result["opacity"] == "1" and result["role"] == "dialog" and result["modal"] == "true" and result["focusClose"]
        and result["imageFits"] and result["wheelZoom"] != "none" and abs(result["dragPanned"]) > 10
        and result["stillOpenAfterDrag"] and abs(result["resetScale"] - 1) < 1e-3 and result["focusTrapped"]
        and result["escCloses"] and result["focusReturned"] and result["outsideClickCloses"]
        and result["clickImageStaysOpen"] and result["dblclickZooms"] and result["dblclickAgainResets"]
        and result["dragOnImageStaysOpen"] and result["scrimClickClosesAfter"]
    )
    if not ok:
        failures.append(f"lightbox {vw['width']}: {result}")
    return result


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    summary, failures = {}, []
    with sync_playwright() as p:
        browser = p.chromium.launch(executable_path=CHROME, args=["--no-sandbox"])
        for width, height in ((1440, 900), (390, 844)):
            for name, route in ROUTES.items():
                for motion in ("reduce", "no-preference"):
                    ctx = browser.new_context(viewport={"width": width, "height": height}, reduced_motion=motion)
                    page = ctx.new_page()
                    page.add_init_script(CLS_INIT)
                    errors = []
                    page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
                    page.on("pageerror", lambda e: errors.append(str(e)))
                    page.on("requestfailed", lambda r: errors.append(f"failed {r.url}"))
                    page.on("response", lambda r: errors.append(f"{r.status} {r.url}") if r.status >= 400 and not r.url.endswith("404.html") else None)
                    page.goto(BASE + route, wait_until="networkidle")
                    page.evaluate("document.fonts.ready")
                    tag = f"{name}-{width}"
                    if motion == "reduce":
                        # Load lazy images and let scroll reveals settle before measuring.
                        page.evaluate("async () => { for (let y = 0; y < document.body.scrollHeight; y += 600) { window.scrollTo(0, y); await new Promise(r => setTimeout(r, 60)); } window.scrollTo(0, 0); }")
                        page.wait_for_load_state("networkidle")
                        page.screenshot(path=str(OUT / f"{tag}-full.png"), full_page=True)
                        audit = page.evaluate(AUDIT_JS)
                        audit["errors"] = errors
                        if name != "404":
                            audit["enhanced"] = page.evaluate("document.getElementById('main')?.dataset.enhancementsReady === 'true'")
                        summary[tag] = audit
                        if audit["overflow"] > 0:
                            failures.append(f"{tag}: horizontal overflow {audit['overflow']}px")
                        if audit["lowContrast"]:
                            failures.append(f"{tag}: contrast {audit['lowContrast']}")
                        if width == 390 and audit["smallTargets"]:
                            failures.append(f"{tag}: small targets {audit['smallTargets']}")
                        if errors:
                            failures.append(f"{tag}: errors {errors}")
                        if any(not i["ok"] or not i["hasSize"] or not i["alt"] for i in audit["images"]):
                            failures.append(f"{tag}: image issue {audit['images']}")
                        if name == "index":
                            summary[tag]["lightbox"] = lightbox_check(page, failures)
                    else:
                        page.wait_for_timeout(4800)
                        page.screenshot(path=str(OUT / f"{tag}-motion-0.png"))
                        total = page.evaluate("document.documentElement.scrollHeight")
                        for i, y in enumerate(range(height, min(total, height * 5), int(height * 0.9)), 1):
                            page.mouse.wheel(0, int(height * 0.9))
                            page.wait_for_timeout(700)
                            page.screenshot(path=str(OUT / f"{tag}-motion-{i}.png"))
                        cls = page.evaluate("window.__cls")
                        summary[tag]["clsWithMotion"] = round(cls, 4)
                        if cls > 0.1:
                            failures.append(f"{tag}: CLS {cls}")
                        if errors:
                            failures.append(f"{tag} (motion): errors {errors}")
                    ctx.close()
        summary["lightbox-touch-390"] = touch_check(browser, failures)
        browser.close()
    print(json.dumps(summary, indent=1))
    if failures:
        print("FAILURES:\n" + "\n".join(failures))
        sys.exit(1)
    print(f"Browser audit passed: {len(summary)} route/width checks, lightbox at 1440 and 390.")


if __name__ == "__main__":
    main()
