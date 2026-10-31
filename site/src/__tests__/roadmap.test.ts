import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { milestones, projectUpdates, roadmapSnapshot } from '../data/roadmap';
import { escapeHtml, renderRoadmap } from '../logic/roadmapMarkup';
import { renderSite } from '../logic/siteMarkup';
const template = readFileSync(new URL('../../roadmap.html', import.meta.url), 'utf8');

describe('static public pages', () => {
  it('renders all stable milestone anchors with a compact Later group', () => {
    const html = renderRoadmap(template);
    expect(html).not.toContain('<!-- roadmap:');
    expect(html.match(/class="milestone milestone-/g)).toHaveLength(5);
    for (const m of milestones) {
      expect(html).toContain(`id="${m.id}"`);
      expect(html).toContain(escapeHtml(m.title));
      expect(html).toContain(escapeHtml(m.detail));
    }
    expect(milestones.find((m) => m.id === 'm4')?.status).toBe('in-progress');
    expect(milestones.find((m) => m.id === 'm3')?.statusLabel).toBe('Prototype tested');
    expect(
      milestones
        .filter((m) => ['m1', 'm2', 'm3'].includes(m.id))
        .every((m) => m.status === 'complete'),
    ).toBe(true);
    expect(
      milestones
        .filter((m) => ['m5', 'm6', 'm7'].includes(m.id))
        .every((m) => m.status === 'planned'),
    ).toBe(true);
    expect(roadmapSnapshot.verifiedAt).toBe('2026-10-31');
    expect(roadmapSnapshot.currentSummary).toMatch(
      /synthetic Rekordbox XML.*read-only preview.*new file.*guarded edits/i,
    );
    expect(roadmapSnapshot.unfinished).toMatch(
      /real Rekordbox or OneLibrary.*not been demonstrated/i,
    );
    expect(roadmapSnapshot.unfinished).toMatch(
      /desktop, accessibility and platform testing.*pending/i,
    );
    expect(roadmapSnapshot.unfinished).toMatch(/500,000 tracks and 60 frames per second/i);
    expect(milestones.find((m) => m.id === 'm4')?.detail).toMatch(
      /synthetic Rekordbox XML fixtures.*read-only preview.*new file.*guarded edits/i,
    );
    expect(milestones.find((m) => m.id === 'm4')?.detail).toMatch(
      /compatibility with real Rekordbox or OneLibrary.*not been demonstrated/i,
    );
    expect(projectUpdates).toHaveLength(3);
    expect(projectUpdates.map((u) => u.date)).toEqual(['2026-10-29', '2026-10-27', '2026-10-26']);
    expect(projectUpdates.every((u) => /\/commit\/[0-9a-f]{7,40}$/.test(u.source.href))).toBe(true);
    expect(html).not.toMatch(/<dt[^>]*>(Progress|Next)<\/dt>/);
    for (const u of projectUpdates) expect(html).toContain(escapeHtml(u.source.href));
  });
  it('uses one shared status, focus, date and navigation on every public route without scripts', () => {
    for (const page of ['index.html', 'roadmap.html', 'dossier.html']) {
      const shell = readFileSync(new URL(`../../${page}`, import.meta.url), 'utf8');
      const html = renderSite(renderRoadmap(shell), page);
      expect(html).not.toMatch(/<!-- (site|roadmap):/);
      expect(html.match(/class="status-callout"/g)).toHaveLength(1);
      expect(html).toContain(escapeHtml(roadmapSnapshot.currentFocus));
      expect(html).toContain(roadmapSnapshot.verifiedAt);
      expect(html).toContain(escapeHtml(roadmapSnapshot.working));
      expect(html).toContain(escapeHtml(roadmapSnapshot.unfinished));
      const header = html.match(/<header[\s\S]*?<\/header>/)?.[0] ?? '';
      for (const name of ['Home', 'Roadmap', 'Updates', 'Source'])
        expect(header).toContain(`>${name}</a>`);
      expect(header).not.toContain('dossier.html');
      expect(html).toContain('Classified file (just for fun)');
      for (const meta of ['og:title', 'og:description', 'og:image', 'og:url', 'twitter:card'])
        expect(html).toContain(meta);
      expect(html).not.toMatch(/fonts\.googleapis|fonts\.gstatic/);
      const copy = html
        .replace(/<head[\s\S]*?<\/head>|<!--[\s\S]*?-->/g, '')
        .replace(/<[^>]+>/g, ' ');
      expect(copy).not.toMatch(/[\u2013\u2014\u2018\u2019\u201c\u201d\u2026\u2192]/);
      expect(copy).not.toMatch(
        /smoke|admission|owner-authorized|fail-closed|qualification|Bot Screen|ADR-005|A1-A3|C1-C6/i,
      );
    }
  });
  it('escapes markup and rejects unknown fragment names', () => {
    expect(escapeHtml(`<script title="x">&'</script>`)).toBe(
      '&lt;script title=&quot;x&quot;&gt;&amp;&#39;&lt;/script&gt;',
    );
    expect(() => renderRoadmap('<!-- roadmap:missing -->')).toThrow('Unknown roadmap fragment');
    expect(() => renderSite('<!-- site:missing -->', 'index.html')).toThrow(
      'Unknown site fragment',
    );
    expect(() => renderSite('', 'unknown.html')).toThrow('Unknown site page');
  });
  it('is idempotent', () => {
    const html = renderSite(renderRoadmap(template), 'roadmap.html');
    expect(renderSite(renderRoadmap(html), 'roadmap.html')).toBe(html);
  });
});
