import { roadmapSnapshot, screenshots, type Screenshot } from '../data/roadmap';
import { escapeHtml } from './roadmapMarkup';
import { renderWaveSvg } from './waveArt';

/**
 * A screenshot figure. The link opens the native-resolution image directly when
 * scripts are off; with scripts the lightbox intercepts it. Width/height reserve
 * space (no layout shift); srcset lets 1x screens take the half-width copy.
 */
export function renderShot(shot: Screenshot, sizes: string, eager = false): string {
  const full = `./images/${shot.id}.webp`;
  const srcset = shot.halfWidth
    ? ` srcset="./images/${shot.id}-${shot.halfWidth}.webp ${shot.halfWidth}w, ${full} ${shot.width}w" sizes="${sizes}"`
    : '';
  const loading = eager ? ' fetchpriority="high"' : ' loading="lazy"';
  return `<figure class="shot shot-${shot.id}"><a class="shot-link" href="${full}" data-lightbox data-width="${shot.width}" data-height="${shot.height}"><img src="${full}"${srcset} width="${shot.width}" height="${shot.height}" alt="${escapeHtml(shot.alt)}" decoding="async"${loading} /><span class="shot-zoom" aria-hidden="true"><svg viewBox="0 0 24 24" width="18" height="18" focusable="false"><circle cx="10.5" cy="10.5" r="6.5" /><path d="M15.5 15.5L20 20M10.5 7.5v6M7.5 10.5h6" /></svg></span><span class="visually-hidden"> Opens a larger, zoomable view.</span></a><figcaption>${escapeHtml(shot.caption)}</figcaption></figure>`;
}

const pages: Record<string, { title: string; description: string }> = {
  'index.html': {
    title: 'OxiDJ | Your music. Your decks.',
    description:
      'A Rust-first DJ library for local collections, with imported-playlist browsing and two playable decks. See what works today, try the catalog CLI and follow Rekordbox XML development.',
  },
  'roadmap.html': {
    title: 'OxiDJ | Roadmap and updates',
    description:
      'Follow the local catalog, imported-playlist browsing, reference-only import, prototype viewer, Rekordbox XML groundwork and the first two decks. See the milestones and recent changes.',
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
/** Gallery column widths: the wide (7fr) and narrow (5fr) columns at the full container. */
const wide = '(min-width: 1360px) 680px, 92vw';
const narrow = '(min-width: 1360px) 480px, 92vw';
/** Break the shared status at its topic transitions without hiding any qualifications. */
function statusParagraphs(text: string): string {
  return text
    .split(
      / (?=The window has|Tracks load|Every deck section|From the command line|Computed BPM|The decks have|Production desktop)/,
    )
    .map((paragraph) => `<p>${escapeHtml(paragraph)}</p>`)
    .join('');
}
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
    head: `<link rel="preload" href="./fonts/epilogue-400.woff2" as="font" type="font/woff2" crossorigin /><link rel="preload" href="./fonts/epilogue-700.woff2" as="font" type="font/woff2" crossorigin /><meta name="description" content="${escapeHtml(info.description)}" /><title>${escapeHtml(info.title)}</title><link rel="canonical" href="${canonical}" /><meta property="og:type" content="website" /><meta property="og:title" content="${escapeHtml(info.title)}" /><meta property="og:description" content="${escapeHtml(info.description)}" /><meta property="og:image" content="${origin}images/social.png" /><meta property="og:image:width" content="1200" /><meta property="og:image:height" content="630" /><meta property="og:image:alt" content="Your music. Your decks. Provisional OxiDJ library window with two decks and synthetic tracks." /><meta property="og:url" content="${canonical}" /><meta name="twitter:card" content="summary_large_image" /><meta name="twitter:title" content="${escapeHtml(info.title)}" /><meta name="twitter:description" content="${escapeHtml(info.description)}" /><meta name="twitter:image" content="${origin}images/social.png" />`,
    header: `<a class="skip-link" href="${page === '404.html' ? './404.html#main' : '#main'}">Skip to content</a><header class="site-header"><div class="site-header-inner">${logo}<nav aria-label="Main navigation">${nav}</nav></div></header>`,
    footer: `<footer class="site-footer"><div class="footer-inner"><div class="footer-mast">${logo}<p>Current focus: ${escapeHtml(roadmapSnapshot.currentFocus)}.</p></div><nav aria-label="Footer navigation">${nav}<a href="./dossier.html">Classified file (just for fun)</a></nav><p class="snapshot">Snapshot <time datetime="${roadmapSnapshot.verifiedAt}">${roadmapSnapshot.verifiedAt}</time>. Site code MIT. Fonts under the SIL Open Font License. Site screenshots use a synthetic catalog except for one cropped owner-library row, shown as metadata only; that track was not loaded or played. Built by <a href="https://soundcloud.com/s3nci">Senči on SoundCloud</a>.</p></div></footer>`,
    status: `<aside class="status-callout reveal" aria-labelledby="status-title"><div class="status-head"><h2 id="status-title">Status: pre-release</h2><p>Snapshot <time datetime="${roadmapSnapshot.verifiedAt}">${roadmapSnapshot.verifiedAt}</time>. Current focus: <a href="./roadmap.html#m8">${escapeHtml(roadmapSnapshot.currentFocus)}</a>.</p></div><div class="status-cols"><section class="status-works" aria-labelledby="status-works"><h3 id="status-works">Works today</h3>${statusParagraphs(roadmapSnapshot.working)}</section><section class="status-open" aria-labelledby="status-open"><h3 id="status-open">Not yet demonstrated</h3>${statusParagraphs(roadmapSnapshot.unfinished)}</section></div></aside>`,
    focus: escapeHtml(roadmapSnapshot.currentFocus),
    wave: renderWaveSvg(),
    stage: renderShot(screenshots.overview, '(min-width: 1360px) 1200px, 92vw', true),
    deckMixerV2: renderShot(screenshots.deckMixerV2, '(min-width: 1360px) 1200px, 92vw'),
    layoutB: renderShot(screenshots.layoutB, '(min-width: 960px) 560px, 92vw'),
    layoutA: renderShot(screenshots.layoutA, '(min-width: 960px) 560px, 92vw'),
    layoutC: renderShot(screenshots.layoutC, '(min-width: 960px) 560px, 92vw'),
    layoutD: renderShot(screenshots.layoutD, '(min-width: 960px) 560px, 92vw'),
    shotMerged: renderShot(screenshots.decksMerged, wide),
    shotMixer: renderShot(screenshots.decksMixer, wide),
    shotBandsBefore: renderShot(screenshots.bandsBefore, wide),
    shotBandsAfter: renderShot(screenshots.bandsAfter, wide),
    shotFallback: renderShot(screenshots.gridFallback, wide),
    shotCollapsed: renderShot(screenshots.decksCollapsed, wide),
    shotSynced: renderShot(screenshots.decksSynced, narrow),
    shotRefused: renderShot(screenshots.syncRefused, narrow),
    shotImportedPlaylists: renderShot(screenshots.importedPlaylists, '240px'),
    shotOwnerLibraryRecord: renderShot(
      screenshots.ownerLibraryRecord,
      '(min-width: 1360px) 838px, 92vw',
    ),
    shotWaveGrid: renderShot(screenshots.waveformGrid, '359px'),
    shotWaveClick: renderShot(screenshots.waveformClick, '359px'),
  };
  return html.replace(/<!-- site:(\w+) -->/g, (_, key: string) => {
    if (fragments[key] === undefined) throw new Error(`Unknown site fragment: ${key}`);
    return fragments[key];
  });
}
