import { roadmapSnapshot } from '../data/roadmap';
import { escapeHtml } from './roadmapMarkup';

const pages: Record<string, { title: string; description: string }> = {
  'index.html': {
    title: 'OxiDJ | A library before the decks',
    description:
      'A Rust-first DJ library for local collections. See what works today, try the catalog CLI and follow Rekordbox XML development.',
  },
  'roadmap.html': {
    title: 'OxiDJ | Roadmap and updates',
    description:
      'Follow the local catalog, reference-only import, prototype viewer and current Rekordbox XML groundwork. See the milestones and recent changes.',
  },
  'dossier.html': {
    title: 'OxiDJ | Classified file 0042',
    description:
      'A playful classified file for your DJ library, with the real OxiDJ development status kept in plain sight.',
  },
  '404.html': {
    title: 'OxiDJ | Page missing',
    description: 'This file is missing. Find the OxiDJ home page, roadmap and development updates.',
  },
};
const origin = 'https://senci.github.io/oxidj/';
export function renderSite(html: string, page: string): string {
  const info = pages[page];
  if (!info) throw new Error(`Unknown site page: ${page}`);
  const canonical = origin + (page === 'index.html' ? '' : page);
  const nav = [
    ['Home', './'],
    ['Roadmap', './roadmap.html'],
    ['Updates', './roadmap.html#updates'],
    ['Source', roadmapSnapshot.repo],
  ]
    .map(
      ([name, href]) =>
        `<a href="${href}"${(page === 'index.html' && name === 'Home') || (page === 'roadmap.html' && name === 'Roadmap') ? ' aria-current="page"' : ''}>${name}</a>`,
    )
    .join('');
  const logo =
    '<a class="wordmark" href="./" aria-label="OxiDJ home"><span class="wordmark-mark" aria-hidden="true">/</span>OxiDJ</a>';
  const fragments: Record<string, string> = {
    head: `<meta name="description" content="${escapeHtml(info.description)}" /><title>${escapeHtml(info.title)}</title><link rel="canonical" href="${canonical}" /><meta property="og:type" content="website" /><meta property="og:title" content="${escapeHtml(info.title)}" /><meta property="og:description" content="${escapeHtml(info.description)}" /><meta property="og:image" content="${origin}images/social.png" /><meta property="og:image:width" content="1200" /><meta property="og:image:height" content="630" /><meta property="og:image:alt" content="OxiDJ. A library before the decks. Prototype library viewer." /><meta property="og:url" content="${canonical}" /><meta name="twitter:card" content="summary_large_image" /><meta name="twitter:title" content="${escapeHtml(info.title)}" /><meta name="twitter:description" content="${escapeHtml(info.description)}" /><meta name="twitter:image" content="${origin}images/social.png" />`,
    header: `<a class="skip-link" href="${page === '404.html' ? './404.html#main' : '#main'}">Skip to content</a><header class="site-header">${logo}<nav aria-label="Main navigation">${nav}</nav></header>`,
    footer: `<footer class="site-footer">${logo}<p>Current focus: ${escapeHtml(roadmapSnapshot.currentFocus)}.</p><p class="snapshot">Snapshot: <time datetime="${roadmapSnapshot.verifiedAt}">${roadmapSnapshot.verifiedAt}</time>. Site code: MIT. Fonts: SIL Open Font License.</p><nav aria-label="Footer navigation">${nav}<a href="./dossier.html">Classified file (just for fun)</a></nav></footer>`,
    status: `<aside class="status-callout" aria-labelledby="status-title"><h2 id="status-title">Status: pre-release</h2><p>${escapeHtml(roadmapSnapshot.working)}</p><p>${escapeHtml(roadmapSnapshot.unfinished)}</p><p>Current focus: <a href="./roadmap.html#m4">${escapeHtml(roadmapSnapshot.currentFocus)}</a>. Snapshot: <time datetime="${roadmapSnapshot.verifiedAt}">${roadmapSnapshot.verifiedAt}</time>.</p></aside>`,
    focus: escapeHtml(roadmapSnapshot.currentFocus),
  };
  return html.replace(/<!-- site:(\w+) -->/g, (_, key: string) => {
    if (fragments[key] === undefined) throw new Error(`Unknown site fragment: ${key}`);
    return fragments[key];
  });
}
