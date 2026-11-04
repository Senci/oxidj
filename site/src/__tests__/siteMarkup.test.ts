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
    expect(html).toContain(roadmapSnapshot.working);
    expect(html).toContain(roadmapSnapshot.unfinished);
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
    expect(html).toContain('data-width="1440" data-height="791"');
    expect(html).toContain(
      'srcset="./images/app-overview-720.webp 720w, ./images/app-overview.webp 1440w"',
    );
    expect(html).toContain('width="1440" height="791"');
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
  it('places the stage, gallery and waveform art on the home page only', () => {
    const home = renderSite(
      renderRoadmap(readFileSync(new URL('../../index.html', import.meta.url), 'utf8')),
      'index.html',
    );
    expect(home.match(/data-lightbox/g)).toHaveLength(5);
    expect(home).toContain('class="wave"');
    expect(home.match(/class="rail-stop /g)).toHaveLength(8);
    for (const shot of Object.values(screenshots)) {
      expect(shot.alt.length).toBeGreaterThan(20);
      expect(home).toContain(`./images/${shot.id}.webp`);
    }
    expect(home).not.toMatch(/library-prototype|\/home\/|waveform-mono|waveform-3band/);
  });
});
