import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const links = new Set();
for (const page of ['index', 'roadmap', 'dossier']) {
  for (const [, href] of readFileSync(`dist/${page}.html`, 'utf8').matchAll(
    /href="(https:\/\/github\.com\/[^" ]+)"/g,
  ))
    links.add(href);
}
for (const href of links) {
  const response = await fetch(href, { signal: AbortSignal.timeout(30000) });
  assert.equal(response.status, 200, `${href}: HTTP ${response.status}`);
  const url = new URL(href);
  if (url.hash) {
    const path = url.pathname.split('/blob/master/')[1];
    const md = readFileSync(`../${path}`, 'utf8');
    const slugs = [...md.matchAll(/^#{1,6} (.+)$/gm)].map(([, heading]) =>
      heading
        .toLowerCase()
        .replace(/[^\p{L}\p{N}_\- ]/gu, '')
        .replaceAll(' ', '-'),
    );
    assert.ok(
      slugs.includes(decodeURIComponent(url.hash.slice(1))),
      `Missing source heading: ${href}`,
    );
  }
  console.log(`HTTP 200${url.hash ? ', source heading verified' : ''}: ${href}`);
  await response.body?.cancel();
}
console.log(`External links verified: ${links.size}.`);
