import { pathToFileURL } from 'node:url';
// Optional local verification tooling; no browser dependency is added to the site.
const { default: puppeteer } = await import(
  pathToFileURL(`${process.env.HALLMARK_PUPPETEER}/lib/puppeteer/puppeteer-core.js`).href
);
const browser = await puppeteer.launch({
  executablePath: process.env.CHROME_PATH || '/usr/bin/chromium',
  headless: true,
  args: ['--no-sandbox'],
});
try {
  const page = await browser.newPage();
  await page.setViewport({ width: 1200, height: 630, deviceScaleFactor: 1 });
  await page.goto('http://127.0.0.1:5176/', { waitUntil: 'networkidle0' });
  await page.evaluate(async () => {
    await document.fonts.ready;
    document
      .querySelectorAll('body > :not(main), main > :not(.hero)')
      .forEach((node) => node.remove());
    const hero = document.querySelector('.hero');
    hero.style.height = '630px';
    hero.querySelector('.actions').remove();
  });
  await page.screenshot({ path: 'public/images/social.png' });
  console.log('Social image: 1200x630, actual home hero and prototype screenshot.');
} finally {
  await browser.close();
}
