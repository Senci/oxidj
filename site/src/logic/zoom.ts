/**
 * Pure zoom/pan math for the screenshot lightbox.
 *
 * The image is laid out at its fitted size, centred in the viewport, and moved
 * with `translate(x, y) scale(scale)` around its own centre. All points are
 * measured relative to the viewport centre, so the image point under a cursor
 * at `p` is `(p - t) / scale` and stays fixed when zooming at `p`.
 */

export interface View {
  /** Viewport width and height in CSS pixels. */
  readonly width: number;
  readonly height: number;
  /** Fitted (scale 1) image width and height in CSS pixels. */
  readonly imageWidth: number;
  readonly imageHeight: number;
}

export interface ZoomState {
  readonly scale: number;
  readonly x: number;
  readonly y: number;
}

export interface Point {
  readonly x: number;
  readonly y: number;
}

export const MIN_SCALE = 1;
// Captures are 1x screen pixels; beyond 4x a zoom only magnifies blur.
export const MAX_SCALE = 4;
export const IDENTITY: ZoomState = { scale: 1, x: 0, y: 0 };

export function clampScale(scale: number, min = MIN_SCALE, max = MAX_SCALE): number {
  if (!Number.isFinite(scale)) return min;
  return Math.min(max, Math.max(min, scale));
}

/** Largest allowed offset on each axis: the image may slide until its edge meets the viewport edge. */
export function panLimits(scale: number, view: View): Point {
  return {
    x: Math.max(0, (view.imageWidth * scale - view.width) / 2),
    y: Math.max(0, (view.imageHeight * scale - view.height) / 2),
  };
}

function clampAxis(value: number, limit: number): number {
  // Normalise -0 so callers can compare states with toEqual.
  return Math.min(limit, Math.max(-limit, value)) + 0;
}

export function clampPan(state: ZoomState, view: View): ZoomState {
  const limit = panLimits(state.scale, view);
  return { scale: state.scale, x: clampAxis(state.x, limit.x), y: clampAxis(state.y, limit.y) };
}

/** Zoom by `factor`, keeping the image point under `point` (relative to the viewport centre) fixed. */
export function zoomAt(state: ZoomState, factor: number, point: Point, view: View): ZoomState {
  const scale = clampScale(state.scale * factor);
  const ratio = scale / state.scale;
  return clampPan(
    {
      scale,
      x: point.x - (point.x - state.x) * ratio,
      y: point.y - (point.y - state.y) * ratio,
    },
    view,
  );
}

/** Zoom to an absolute scale at `point`. */
export function zoomTo(state: ZoomState, scale: number, point: Point, view: View): ZoomState {
  return zoomAt(state, clampScale(scale) / state.scale, point, view);
}

export function panBy(state: ZoomState, dx: number, dy: number, view: View): ZoomState {
  return clampPan({ scale: state.scale, x: state.x + dx, y: state.y + dy }, view);
}

/** Wheel delta (pixels) to a multiplicative zoom factor; negative delta zooms in. */
export function wheelFactor(deltaY: number): number {
  return Math.exp(-deltaY * 0.0015);
}

/**
 * Fitted image size: as large as the viewport allows after `padding`, but never
 * more than `maxUpscale` times the natural size, so small crops stay crisp.
 */
export function fitImage(
  naturalWidth: number,
  naturalHeight: number,
  viewportWidth: number,
  viewportHeight: number,
  padding = 48,
  maxUpscale = 2,
): { width: number; height: number } {
  const availableWidth = Math.max(1, viewportWidth - padding * 2);
  const availableHeight = Math.max(1, viewportHeight - padding * 2);
  const ratio = Math.min(
    availableWidth / naturalWidth,
    availableHeight / naturalHeight,
    maxUpscale,
  );
  return { width: Math.round(naturalWidth * ratio), height: Math.round(naturalHeight * ratio) };
}

export function toTransform(state: ZoomState): string {
  return `translate3d(${state.x.toFixed(2)}px, ${state.y.toFixed(2)}px, 0) scale(${state.scale.toFixed(4)})`;
}
