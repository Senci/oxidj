import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { milestones, productGoals, projectUpdates, roadmapSnapshot } from '../data/roadmap';
import { escapeHtml, renderRoadmap } from '../logic/roadmapMarkup';

const template = readFileSync(new URL('../../roadmap.html', import.meta.url), 'utf8');

describe('static roadmap', () => {
  it('shows current M4 work in the no-JavaScript roadmap shell', () => {
    const shell = template.replace(/\s+/g, ' ');
    expect(shell).toContain('M4 XML groundwork in progress.');
    expect(shell).toContain('full vendor compatibility remains unpassed');
    expect(shell).toContain('Bounded local acceptance, not production qualification.');
  });

  it('records bounded M4 implementation without promoting qualification or reopening M1–M3', () => {
    for (const id of ['M1', 'M2', 'M3']) {
      expect(milestones.find((milestone) => milestone.id === id)?.status).toBe('complete');
    }
    const m4 = milestones.find((milestone) => milestone.id === 'M4');
    if (!m4) throw new Error('Missing M4 milestone');
    expect(m4.status).toBe('in-progress');
    expect(m4.statusLabel).toBe('In progress');
    for (const implemented of [
      'A1 synthetic XML fixtures',
      'read-only Rekordbox XML preview',
      'bounded fail-closed, no-edit subset export',
    ]) {
      expect(m4.progress).toContain(implemented);
    }
    expect(m4.progress).toContain('not full Rekordbox app or OneLibrary compatibility');
    expect(m4.next).toContain('ADR-005 promotion remain unpassed');
    expect(m4.next).toContain('A1–A3/C1–C6 qualification remains OPEN / NOT QUALIFYING');
    expect(m4.next).toContain('Production GUI admission and 500k/60fps gates remain separate');
    expect(m4.source.href).toMatch(/\/plan\/STATUS\.md$/);
    expect(roadmapSnapshot.currentSummary).toContain('M4 is in progress');
    expect(roadmapSnapshot.currentFocus).toContain('M4');
    expect(roadmapSnapshot.qualificationSummary).toContain('deferred and unverified—not passed');
    expect(projectUpdates[0]?.category).toBe('M4 · XML groundwork');
    expect(projectUpdates[0]?.detail).toContain('qualification remain unpassed');
  });

  it('renders every typed milestone and evidence link without client JavaScript', () => {
    const html = renderRoadmap(template);
    expect(html).not.toContain('<!-- roadmap:');
    expect(html.match(/class="milestone milestone-/g)).toHaveLength(milestones.length);
    for (const milestone of milestones) {
      for (const value of [
        milestone.title,
        milestone.goal,
        milestone.progress,
        milestone.next,
        milestone.statusLabel,
        milestone.source.href,
      ]) {
        expect(html).toContain(escapeHtml(value));
      }
    }
  });

  it('preserves every update, goal and snapshot caveat', () => {
    const html = renderRoadmap(template);
    for (const update of projectUpdates) {
      expect(html).toContain(escapeHtml(update.title));
      expect(html).toContain(escapeHtml(update.detail));
      expect(html).toContain(escapeHtml(update.source.href));
    }
    for (const goal of productGoals) expect(html).toContain(escapeHtml(goal));
    expect(html).toContain(escapeHtml(roadmapSnapshot.currentSummary));
    expect(html).toContain(escapeHtml(roadmapSnapshot.qualificationSummary));
    expect(html).toContain(escapeHtml(roadmapSnapshot.verifiedAt));
    expect(html).not.toContain('Close the catalog acceptance gaps');
  });

  it('escapes markup and fails on a misspelled template fragment', () => {
    expect(escapeHtml(`<script title="x">&'</script>`)).toBe(
      '&lt;script title=&quot;x&quot;&gt;&amp;&#39;&lt;/script&gt;',
    );
    expect(() => renderRoadmap('<!-- roadmap:missing -->')).toThrow('Unknown roadmap fragment');
  });

  it('leaves other HTML unchanged and is idempotent', () => {
    expect(renderRoadmap('<p>Existing dossier</p>')).toBe('<p>Existing dossier</p>');
    const html = renderRoadmap(template);
    expect(renderRoadmap(html)).toBe(html);
  });
});
