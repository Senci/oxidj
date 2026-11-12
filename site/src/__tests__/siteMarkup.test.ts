import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { renderShot, renderSite } from '../logic/siteMarkup';
import { renderRoadmap } from '../logic/roadmapMarkup';
import { roadmapSnapshot, screenshots } from '../data/roadmap';

describe('shared public pages', () => {
  for (const page of ['index.html', 'roadmap.html', 'dossier.html', '404.html']) {
    it(`renders shared chrome and distinct metadata for ${page}`, () => {
      const html = renderSite('<!-- site:head --><!-- site:header --><!-- site:footer -->', page);
      expect(html).not.toContain('<!-- site:');
      expect(html).toContain('rel="canonical"');
      expect(html).toContain('property="og:description"');
      expect(html).toContain('name="twitter:card"');
      expect(html).toContain('Classified file (just for fun)');
      expect(html).toContain('Senči on SoundCloud');
      expect(html).toContain('href="https://soundcloud.com/s3nci"');
      expect(html).toContain(roadmapSnapshot.currentFocus);
      expect(html).toContain(roadmapSnapshot.verifiedAt);
      expect(html).toContain('aria-label="Main navigation"');
      expect(html).toContain('./roadmap.html#updates');
      expect(html).toContain(`https://senci.github.io/oxidj/${page === 'index.html' ? '' : page}`);
    });
  }
  it('uses one status fragment with the shared working and unfinished facts', () => {
    const html = renderSite('<!-- site:status -->', 'index.html');
    expect(html.match(/class="status-callout[ "]/g)).toHaveLength(1);
    expect(html).toContain('Works today');
    expect(html).toContain('Not yet demonstrated');
    expect(html.replaceAll('</p><p>', ' ')).toContain(roadmapSnapshot.working);
    expect(html.replaceAll('</p><p>', ' ')).toContain(roadmapSnapshot.unfinished);
    expect(html).toContain('Status: pre-release');
  });
  it('rejects unknown pages and fragments', () => {
    expect(() => renderSite('', 'missing.html')).toThrow('Unknown site page');
    expect(() => renderSite('<!-- site:missing -->', 'index.html')).toThrow(
      'Unknown site fragment',
    );
  });
});

describe('screenshots', () => {
  it('renders a sized, described figure whose link opens the full-resolution image', () => {
    const html = renderShot(screenshots.overview, '92vw', true);
    expect(html).toContain('href="./images/app-overview.webp" data-lightbox');
    expect(html).toContain('data-width="1440" data-height="635"');
    expect(html).toContain(
      'srcset="./images/app-overview-720.webp 720w, ./images/app-overview.webp 1440w"',
    );
    expect(html).toContain('width="1440" height="635"');
    expect(html).toContain('fetchpriority="high"');
    expect(html).not.toContain('loading="lazy"');
    expect(html).toContain(`alt="${screenshots.overview.alt}"`);
    expect(html).toContain('Opens a larger, zoomable view.');
  });
  it('lazy-loads secondary crops without a srcset when there is no half-size copy', () => {
    const html = renderShot(screenshots.waveformGrid, '359px');
    expect(html).toContain('loading="lazy"');
    expect(html).not.toContain('srcset');
  });
  it('renders each refreshed arrangement with its correct lightbox and responsive dimensions', () => {
    for (const [shot, height] of [
      [screenshots.layoutA, 628],
      [screenshots.layoutB, 635],
      [screenshots.layoutC, 628],
      [screenshots.layoutD, 473],
    ] as const) {
      const html = renderShot(shot, '92vw');
      expect(html).toContain(`href="./images/${shot.id}.webp" data-lightbox`);
      expect(html).toContain(`data-width="1440" data-height="${height}"`);
      expect(html).toContain(`width="1440" height="${height}"`);
      expect(html).toContain(`./images/${shot.id}-720.webp 720w`);
      expect(html).toContain(`alt="${shot.alt}"`);
    }
  });
  it('pins the imported-playlist capture dimensions, accessible description and lightbox path', () => {
    const html = renderShot(screenshots.importedPlaylists, '240px');
    expect(html).toContain('href="./images/imported-playlist-load-controls.webp" data-lightbox');
    expect(html).toContain('data-width="175" data-height="252"');
    expect(html).toContain('width="175" height="252"');
    expect(html).toContain(`alt="${screenshots.importedPlaylists.alt}"`);
    expect(html).toContain('Load A and Load B');
    expect(html).not.toContain('srcset');
  });

  it('places the stage, gallery and waveform art on the home page only', () => {
    const home = renderSite(
      renderRoadmap(readFileSync(new URL('../../index.html', import.meta.url), 'utf8')),
      'index.html',
    );
    expect(home.match(/data-lightbox/g)).toHaveLength(17);
    expect(home).not.toContain('senci-owner-library-record');
    expect(home).toContain('class="wave"');
    expect(home.match(/class="rail-stop /g)).toHaveLength(9);
    for (const shot of Object.values(screenshots)) {
      expect(shot.alt.length).toBeGreaterThan(20);
      expect(home).toContain(`./images/${shot.id}.webp`);
    }
    expect(home).not.toMatch(
      /library-prototype|\/home\/|waveform-mono|waveform-3band|track-list\.webp|decks-bar\.webp/,
    );
  });
});
