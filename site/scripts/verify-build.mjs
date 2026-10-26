import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

// A static server cannot repair missing entries or root-relative project assets.
const root = new URL('../dist/', import.meta.url);
const origin = new URL('https://pages.example/oxidj/');
const entries = ['index.html', 'roadmap.html', 'dossier.html'];
let checked = 0;
for (const entry of entries) {
  const html = readFileSync(new URL(entry, root), 'utf8');
  assert.match(html, /<html\s+lang="en"/);
  assert.match(html, /<title>[^<]+<\/title>/);
  for (const [, reference] of html.matchAll(/(?:src|href)="([^"]+)"/g)) {
    const url = new URL(reference, new URL(entry, origin));
    if (url.origin !== origin.origin) continue;
    assert.ok(url.pathname.startsWith('/oxidj/'), `${entry}: escaped project base: ${reference}`);
    const relative = url.pathname.slice('/oxidj/'.length) || 'index.html';
    const target = new URL(relative, root);
    assert.ok(existsSync(target), `${entry}: missing ${fileURLToPath(target)}`);
    if (url.hash && relative.endsWith('.html')) {
      const targetHtml = readFileSync(target, 'utf8');
      assert.ok(
        targetHtml.includes(`id="${decodeURIComponent(url.hash.slice(1))}"`),
        `${entry}: missing anchor ${reference}`,
      );
    }
    checked++;
  }
}
const teaser = readFileSync(new URL('index.html', root), 'utf8');
assert.match(teaser, /href="\.\/roadmap\.html"/);
assert.match(teaser, /href="\.\/dossier\.html"/);
assert.doesNotMatch(teaser, /<script\b/); // Essential teaser content is script-independent.
const roadmap = readFileSync(new URL('roadmap.html', root), 'utf8');
assert.doesNotMatch(roadmap, /<!-- roadmap:/);
assert.equal((roadmap.match(/class="milestone milestone-/g) ?? []).length, 8);
assert.match(roadmap, /Bounded local M1–M3 acceptance is complete/);
assert.match(roadmap, /Production GUI admission/);
assert.match(roadmap, /assets\/roadmap-[^" ]+\.css/);
console.log(
  `Pages build verified: ${entries.length} entries, ${checked} local references under /oxidj/, script-free teaser.`,
);
