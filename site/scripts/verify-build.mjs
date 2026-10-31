import assert from 'node:assert/strict';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const root = new URL('../dist/', import.meta.url);
const origin = new URL('https://senci.github.io/oxidj/');
const entries = ['index.html', 'roadmap.html', 'dossier.html', '404.html'];
let checked = 0;
function verifyReference(reference, from) {
  const url = new URL(reference, new URL(from, origin));
  if (url.origin !== origin.origin) return;
  assert.ok(url.pathname.startsWith('/oxidj/'), `${from}: escaped project base: ${reference}`);
  const relative = url.pathname.slice('/oxidj/'.length) || 'index.html';
  const target = new URL(relative, root);
  assert.ok(existsSync(target), `${from}: missing ${fileURLToPath(target)}`);
  if (url.hash && relative.endsWith('.html'))
    assert.ok(
      readFileSync(target, 'utf8').includes(`id="${decodeURIComponent(url.hash.slice(1))}"`),
      `${from}: missing anchor ${reference}`,
    );
  checked++;
}
for (const entry of entries) {
  const html = readFileSync(new URL(entry, root), 'utf8');
  assert.match(html, /<html\s+lang="en"/);
  assert.match(html, /<title>[^<]+<\/title>/);
  assert.doesNotMatch(html, /<!-- (site|roadmap):|fonts\.googleapis|fonts\.gstatic/);
  for (const [, reference] of html.matchAll(/(?:src|href)="([^"]+)"/g))
    verifyReference(reference, entry);
  for (const tag of ['og:title', 'og:description', 'og:image', 'og:url', 'twitter:card'])
    assert.ok(html.includes(tag), `${entry}: missing ${tag}`);
  assert.match(html, /rel="canonical"/);
  if (entry !== '404.html') assert.equal((html.match(/class="status-callout"/g) ?? []).length, 1);
}
for (const css of readdirSync(new URL('assets/', root)).filter((file) => file.endsWith('.css'))) {
  for (const [, reference] of readFileSync(new URL(`assets/${css}`, root), 'utf8').matchAll(
    /url\(["']?([^)'"\s]+)["']?\)/g,
  ))
    verifyReference(reference, `assets/${css}`);
}
const home = readFileSync(new URL('index.html', root), 'utf8');
const homeScripts = home.match(/<script[^>]*>/g) ?? [];
assert.equal(homeScripts.length, 1, 'home has one enhancement bundle');
assert.match(homeScripts[0], /type="module"/);
assert.ok(homeScripts[0].includes('src="./assets/'));
assert.doesNotMatch(home, /on(?:click|load|mouseover)=|javascript:/i);
assert.ok(home.includes('<details>') && home.includes('data-trace="flow"'));
for (const command of ['init', 'import', 'list'])
  assert.ok(home.includes(`catalog ${command} --db`));
const roadmap = readFileSync(new URL('roadmap.html', root), 'utf8');
for (let i = 0; i <= 7; i++) assert.ok(roadmap.includes(`id="m${i}"`));
assert.equal((roadmap.match(/class="milestone milestone-/g) ?? []).length, 5);
for (const file of ['robots.txt', 'sitemap.xml', 'images/social.png'])
  assert.ok(existsSync(new URL(file, root)));
const social = readFileSync(new URL('images/social.png', root));
assert.equal(social.readUInt32BE(16), 1200);
assert.equal(social.readUInt32BE(20), 630);
console.log(
  `Pages build verified: ${entries.length} entries, ${checked} local references under /oxidj/, static copy and metadata, eight anchors, self-hosted fonts, 1200x630 social image.`,
);
