import { describe, expect, it } from 'vitest';
import { renderSite } from '../logic/siteMarkup';
import { roadmapSnapshot } from '../data/roadmap';

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
    expect(html.match(/class="status-callout"/g)).toHaveLength(1);
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
