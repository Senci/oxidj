import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
const { default: puppeteer } = await import(
  pathToFileURL(`${process.env.HALLMARK_PUPPETEER}/lib/puppeteer/puppeteer-core.js`).href
);
const base = process.env.SITE_URL || 'http://127.0.0.1:4183/oxidj/';
const output = process.env.SITE_AUDIT_OUTPUT || '.hallmark/final';
mkdirSync(output, { recursive: true });
const browser = await puppeteer.launch({
  executablePath: process.env.CHROME_PATH || '/usr/bin/chromium',
  headless: true,
  args: ['--no-sandbox'],
});
const results = [];
const external = new Set();
try {
  for (const width of [390, 1440]) {
    for (const route of ['', 'roadmap.html', 'dossier.html', '404.html']) {
      const page = await browser.newPage();
      await page.setViewport({ width, height: 900, deviceScaleFactor: 1 });
      const errors = [];
      page.on('pageerror', (e) => errors.push(e.message));
      page.on('console', (e) => {
        if (e.type() === 'error') errors.push(e.text());
      });
      page.on('requestfailed', (req) => errors.push(`${req.url()}: ${req.failure()?.errorText}`));
      page.on('response', (res) => {
        if (res.status() >= 400) errors.push(`${res.status()}: ${res.url()}`);
      });
      await page.goto(base + route, { waitUntil: 'networkidle0' });
      if (route !== '404.html') {
        await page.waitForFunction(
          () => document.getElementById('main')?.dataset.enhancementsReady === 'true',
        );
      }
      await page.evaluate(() => document.fonts.ready);
      const audit = await page.evaluate(() => {
        const context = document
          .createElement('canvas')
          .getContext('2d', { willReadFrequently: true });
        function rgba(color) {
          context.clearRect(0, 0, 1, 1);
          context.fillStyle = color;
          context.fillRect(0, 0, 1, 1);
          return [...context.getImageData(0, 0, 1, 1).data];
        }
        function luminance(rgb) {
          const c = rgb.slice(0, 3).map((v) => {
            const s = v / 255;
            return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
          });
          return c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722;
        }
        const smallText = [
          ...document.querySelectorAll(
            'p, small, figcaption, time, a, dt, dd, li, span, code, button',
          ),
        ].filter(
          (el) => el.textContent.trim() && getComputedStyle(el).fontSize.replace('px', '') < 24,
        );
        const contrast = smallText
          .map((el) => {
            const style = getComputedStyle(el);
            let ancestor = el;
            let background;
            while (ancestor) {
              const bg = rgba(getComputedStyle(ancestor).backgroundColor);
              if (bg[3] === 255) {
                background = bg;
                break;
              }
              ancestor = ancestor.parentElement;
            }
            if (!background) return { ratio: 0, text: el.textContent.slice(0, 50) };
            const colors = [luminance(rgba(style.color)), luminance(background)].sort(
              (a, b) => a - b,
            );
            return {
              ratio: (colors[1] + 0.05) / (colors[0] + 0.05),
              text: el.textContent.trim().slice(0, 50),
            };
          })
          .sort((a, b) => a.ratio - b.ratio);
        return {
          title: document.title,
          overflow: document.documentElement.scrollWidth > innerWidth,
          brokenImages: [...document.images].filter(
            (image) => !image.complete || image.naturalWidth === 0,
          ).length,
          fonts:
            document.fonts.check('600 16px Epilogue') &&
            document.fonts.check('400 16px "IBM Plex Mono"'),
          shortTargets: [...document.querySelectorAll('a,button')]
            .filter(
              (el) =>
                el.getBoundingClientRect().height < 44 || el.getBoundingClientRect().width < 44,
            )
            .map((el) => el.textContent),
          minContrast: contrast[0],
          lowContrast: contrast.filter((item) => item.ratio < 4.5),
          links: [...document.querySelectorAll('a')].map((a) => a.href),
          requests: performance.getEntriesByType('resource').map((r) => r.name),
        };
      });
      assert.equal(errors.length, 0, JSON.stringify(errors));
      assert.equal(audit.overflow, false, `${route} horizontal overflow`);
      assert.equal(audit.brokenImages, 0);
      assert.equal(audit.fonts, true);
      assert.deepEqual(audit.shortTargets, []);
      assert.deepEqual(audit.lowContrast, []);
      assert.ok(
        audit.requests.every((url) => new URL(url).origin === new URL(base).origin),
        'Third-party resource request',
      );
      for (const url of audit.links)
        if (new URL(url).origin !== new URL(base).origin) external.add(url);
      await page.keyboard.press('Tab');
      assert.equal(
        await page.evaluate(() => document.activeElement.textContent),
        'Skip to content',
      );
      await page.keyboard.press('Enter');
      assert.equal(await page.evaluate(() => location.hash), '#main');
      await page.evaluate(() => {
        document.activeElement.blur();
        window.scrollTo({ top: 0, behavior: 'instant' });
      });
      const brokenAnchors = await page.$$eval('a[href^="#"]', (links) =>
        links
          .map((link) => link.getAttribute('href'))
          .filter((href) => href && !document.getElementById(decodeURIComponent(href.slice(1)))),
      );
      assert.deepEqual(brokenAnchors, [], `${route} broken same-page anchor`);
      await page.screenshot({ path: `${output}/${route || 'home'}-${width}.png`, fullPage: true });
      if (route === '') {
        const firstSummary = await page.$('.flow-details summary');
        await firstSummary.focus();
        await page.keyboard.press('Enter');
        assert.equal(
          await page.$eval('.flow-details details', (el) => el.open),
          true,
          'native disclosure opens with Enter',
        );
        await page.keyboard.press('Enter');
        assert.equal(
          await page.$eval('.flow-details details', (el) => el.open),
          false,
          'native disclosure closes with Enter',
        );
        const trace = await page.$('[data-trace="flow"]');
        await trace.focus();
        assert.equal(
          await page.$eval('[data-trace="flow"]', (el) => el.hidden),
          false,
          'trace control enhanced',
        );
        await page.keyboard.press('Space');
        assert.ok(
          (await page.$eval('.flow-trace', (el) => el.getAnimations().length)) > 0,
          'keyboard activation starts finite trace',
        );
        const endpoint = await page.$eval('.flow-trace', (el) => {
          const animation = el.getAnimations()[0];
          animation.currentTime = 699;
          const matrix = new DOMMatrix(getComputedStyle(el).transform);
          return { x: matrix.m41, endpoint: 410 };
        });
        assert.ok(
          Math.abs(endpoint.x - endpoint.endpoint) < 3,
          `trace endpoint ${JSON.stringify(endpoint)}`,
        );
        await page.emulateMediaFeatures([{ name: 'prefers-reduced-motion', value: 'reduce' }]);
        await page.waitForFunction(() => document.querySelector('[data-trace="flow"]').hidden);
        assert.equal(
          await page.$eval('[data-trace="flow"]', (el) => el.hidden),
          true,
          'reduced motion hides optional control',
        );
        assert.equal(
          await page.evaluate(() => document.activeElement.id),
          'why',
          'focus moves to nearby flow heading when reduced motion hides focused control',
        );
        assert.equal(
          await page.$eval('#why', (el) => getComputedStyle(el, ':focus').outlineStyle !== 'none'),
          true,
          'recovered heading has visible focus',
        );
        assert.equal(
          await page.$eval('.reference-flow', (el) => el.classList.contains('is-tracing')),
          false,
          'reduced motion cancels active trace',
        );
      }
      if (route === 'roadmap.html') {
        const trace = await page.$('[data-trace="timeline"]');
        await trace.focus();
        await page.keyboard.press('Enter');
        assert.equal(
          await page.$eval('#m4', (el) => el.classList.contains('milestone-in-progress')),
          true,
          'timeline retains M4 in-progress state',
        );
        assert.ok(
          (await page.$eval('.milestone-trace', (el) => el.getAnimations().length)) > 0,
          'timeline trace runs along the fixed rail',
        );
        await page.emulateMediaFeatures([{ name: 'prefers-reduced-motion', value: 'reduce' }]);
        await page.waitForFunction(() => document.querySelector('[data-trace="timeline"]').hidden);
        assert.equal(
          await page.$eval('[data-trace="timeline"]', (el) => el.hidden),
          true,
          'timeline control hidden for reduced motion',
        );
        assert.equal(
          await page.$eval('#roadmap', (el) => el.classList.contains('is-tracing')),
          false,
          'timeline trace cancelled for reduced motion',
        );
        await page.reload({ waitUntil: 'networkidle0' });
        assert.equal(
          await page.$eval('[data-trace="timeline"]', (el) => el.hidden),
          true,
          'reduced motion respected on load',
        );
      }
      if (route === 'dossier.html') {
        await page.evaluate(() => (document.documentElement.style.scrollBehavior = 'auto'));
        await page.$eval('#asylum-button', (button) =>
          button.scrollIntoView({ block: 'center', behavior: 'instant' }),
        );
        const box = await page.$eval('#asylum-button', (button) => {
          const rect = button.getBoundingClientRect();
          return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
        });
        await page.mouse.move(box.x, box.y);
        await page.mouse.click(box.x, box.y);
        assert.match(await page.$eval('#asylum-output', (el) => el.textContent), /ASYLUM DENIED/);
      }
      await page.setJavaScriptEnabled(false);
      await page.reload({ waitUntil: 'networkidle0' });
      assert.match(
        await page.$eval('main', (el) => el.textContent),
        route === '404.html' ? /This page isn't in the collection/ : /Status: pre-release/,
      );
      results.push({
        route: route || 'home',
        width,
        consoleErrors: errors,
        minContrast: audit.minContrast,
        fonts: audit.fonts,
        noJs: true,
      });
      await page.close();
    }
  }
} finally {
  await browser.close();
}
writeFileSync(
  `${output}/browser-results.json`,
  JSON.stringify({ results, external: [...external] }, null, 2),
);
console.log(JSON.stringify({ results, external: [...external] }, null, 2));
