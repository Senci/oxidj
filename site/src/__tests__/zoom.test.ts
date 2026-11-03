import { describe, expect, it } from 'vitest';
import {
  IDENTITY,
  MAX_SCALE,
  MIN_SCALE,
  clampPan,
  clampScale,
  fitImage,
  panBy,
  panLimits,
  toTransform,
  wheelFactor,
  zoomAt,
  zoomTo,
  type View,
} from '../logic/zoom';

const view: View = { width: 1000, height: 800, imageWidth: 800, imageHeight: 440 };

describe('lightbox zoom math', () => {
  it('clamps scale between the minimum and maximum, including non-finite input', () => {
    expect(clampScale(0.2)).toBe(MIN_SCALE);
    expect(clampScale(100)).toBe(MAX_SCALE);
    expect(clampScale(2.5)).toBe(2.5);
    expect(clampScale(Number.NaN)).toBe(MIN_SCALE);
  });

  it('never zooms out past fit or in past the maximum', () => {
    expect(zoomAt(IDENTITY, 0.5, { x: 100, y: 0 }, view)).toEqual(IDENTITY);
    let state = IDENTITY;
    for (let i = 0; i < 40; i++) state = zoomAt(state, 1.5, { x: 0, y: 0 }, view);
    expect(state.scale).toBe(MAX_SCALE);
  });

  it('keeps the image point under the cursor fixed while zooming', () => {
    const point = { x: 120, y: -60 };
    const before = zoomTo(IDENTITY, 2, { x: 0, y: 0 }, view);
    const imagePoint = {
      x: (point.x - before.x) / before.scale,
      y: (point.y - before.y) / before.scale,
    };
    const after = zoomAt(before, 1.5, point, view);
    expect(after.scale).toBe(3);
    expect(after.x + imagePoint.x * after.scale).toBeCloseTo(point.x, 6);
    expect(after.y + imagePoint.y * after.scale).toBeCloseTo(point.y, 6);
  });

  it('limits panning so an image edge never leaves its viewport edge', () => {
    expect(panLimits(1, view)).toEqual({ x: 0, y: 0 });
    // 800 * 2 = 1600 wide in a 1000 viewport: 300 px of travel each way.
    expect(panLimits(2, view)).toEqual({ x: 300, y: 40 });
    const zoomed = { scale: 2, x: 0, y: 0 };
    expect(panBy(zoomed, 1000, -1000, view)).toEqual({ scale: 2, x: 300, y: -40 });
    expect(panBy(zoomed, -50, 10, view)).toEqual({ scale: 2, x: -50, y: 10 });
    expect(clampPan({ scale: 1, x: 40, y: -40 }, view)).toEqual(IDENTITY);
  });

  it('re-centres when zooming back out to fit', () => {
    const zoomed = panBy(zoomTo(IDENTITY, 3, { x: 200, y: 100 }, view), -150, 0, view);
    expect(zoomTo(zoomed, 1, { x: -300, y: 0 }, view)).toEqual(IDENTITY);
  });

  it('maps wheel deltas to symmetric zoom factors', () => {
    expect(wheelFactor(0)).toBe(1);
    expect(wheelFactor(-100)).toBeGreaterThan(1);
    expect(wheelFactor(-100) * wheelFactor(100)).toBeCloseTo(1, 10);
  });

  it('fits within the padded viewport without upscaling beyond the limit', () => {
    expect(fitImage(1440, 793, 1440, 900, 56)).toEqual({ width: 1328, height: 731 });
    expect(fitImage(359, 405, 1440, 900, 56, 2)).toEqual({ width: 698, height: 788 });
    expect(fitImage(359, 405, 390, 844, 12, 2)).toEqual({ width: 366, height: 413 });
    expect(fitImage(200, 100, 1440, 900, 56, 1)).toEqual({ width: 200, height: 100 });
  });

  it('serialises the state to one composited transform', () => {
    expect(toTransform({ scale: 2, x: 10, y: -5.5 })).toBe(
      'translate3d(10.00px, -5.50px, 0) scale(2.0000)',
    );
  });
});
