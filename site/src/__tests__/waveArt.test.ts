import { describe, expect, it } from 'vitest';
import { envelopePath, prng, renderWaveSvg, waveArt } from '../logic/waveArt';

describe('hero waveform art', () => {
  it('is deterministic for a seed', () => {
    expect(waveArt({ seed: 7 })).toEqual(waveArt({ seed: 7 }));
    expect(waveArt({ seed: 7 }).bands.mid).not.toBe(waveArt({ seed: 8 }).bands.mid);
    const random = prng(1);
    const values = Array.from({ length: 100 }, random);
    expect(values.every((v) => v >= 0 && v < 1)).toBe(true);
  });

  it('draws closed envelopes mirrored around the centre line', () => {
    expect(envelopePath([0.5, 1], 100, 40)).toBe('M0 10L100 0L100 40L0 30Z');
    expect(envelopePath([1], 100, 40)).toBe('');
  });

  it('keeps every band inside the canvas', () => {
    const art = waveArt();
    for (const d of Object.values(art.bands)) {
      const ys = [...d.matchAll(/[ML][\d.]+ ([\d.]+)/g)].map((m) => Number(m[1]));
      expect(ys.length).toBeGreaterThan(100);
      expect(Math.min(...ys)).toBeGreaterThanOrEqual(0);
      expect(Math.max(...ys)).toBeLessThanOrEqual(art.height);
    }
  });

  it('puts a bar line on every downbeat of the 4/4 grid', () => {
    const art = waveArt({ beats: 32 });
    expect(art.beats).toHaveLength(33);
    expect(art.beats.filter((b) => b.bar !== null).map((b) => b.bar)).toEqual([
      1, 2, 3, 4, 5, 6, 7, 8,
    ]);
    const html = renderWaveSvg(art);
    expect(html.match(/class="bar"/g)).toHaveLength(8);
    expect(html.match(/class="beat"/g)).toHaveLength(25);
    expect(html).toContain('aria-hidden="true"');
    expect(html).toContain('<span style="left:50%">Bar 5</span>');
  });
});
