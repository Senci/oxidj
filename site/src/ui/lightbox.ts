import {
  IDENTITY,
  fitImage,
  panBy,
  toTransform,
  wheelFactor,
  zoomAt,
  zoomTo,
  type Point,
  type View,
  type ZoomState,
} from '../logic/zoom';

/**
 * Hand-written screenshot lightbox.
 *
 * Progressive enhancement: every trigger is a plain `<a data-lightbox href="full.webp">`,
 * so without JavaScript the link opens the full-resolution image. With JavaScript
 * a modal dialog shows it with wheel/pinch/double-click/keyboard zoom and drag-to-pan.
 */

export interface Lightbox {
  readonly element: HTMLElement;
  open(trigger: HTMLAnchorElement): void;
  close(): void;
  isOpen(): boolean;
  state(): ZoomState;
}

const KEY_STEP = 1.25;
const PAN_STEP = 64;
const DOUBLE_CLICK_SCALE = 2.5;
const DRAG_THRESHOLD = 4;

const closeIcon =
  '<svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" focusable="false"><path d="M6 6l12 12M18 6L6 18" /></svg>';

function build(doc: Document): {
  overlay: HTMLElement;
  stage: HTMLElement;
  image: HTMLImageElement;
  closeButton: HTMLButtonElement;
} {
  const overlay = doc.createElement('div');
  overlay.className = 'lightbox';
  overlay.setAttribute('role', 'dialog');
  overlay.setAttribute('aria-modal', 'true');
  overlay.setAttribute('aria-label', 'Screenshot viewer');
  overlay.hidden = true;
  // Static, module-owned markup only; trigger data is applied later via properties.
  overlay.innerHTML = `<div class="lightbox-stage" tabindex="0" aria-describedby="lightbox-hint"><img class="lightbox-image" alt="" draggable="false" decoding="async" /></div><p class="lightbox-hint" id="lightbox-hint"><span class="hint-fine">Scroll or double-click to zoom. Drag or use the arrow keys to pan. Press 0 to reset, Esc to close.</span><span class="hint-touch">Pinch to zoom. Drag to pan.</span></p><button class="lightbox-close" type="button" aria-label="Close screenshot">${closeIcon}</button>`;
  doc.body.append(overlay);
  const stage = overlay.querySelector<HTMLElement>('.lightbox-stage');
  const image = overlay.querySelector<HTMLImageElement>('.lightbox-image');
  const closeButton = overlay.querySelector<HTMLButtonElement>('.lightbox-close');
  if (!stage || !image || !closeButton) throw new Error('Lightbox markup incomplete');
  return { overlay, stage, image, closeButton };
}

export function initLightbox(
  doc: Document = document,
  prefersReducedMotion: () => boolean = () =>
    doc.defaultView?.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false,
): Lightbox | null {
  const triggers = [...doc.querySelectorAll<HTMLAnchorElement>('a[data-lightbox]')];
  if (triggers.length === 0) return null;
  const win = doc.defaultView ?? window;
  const { overlay, stage, image, closeButton } = build(doc);

  let zoom: ZoomState = IDENTITY;
  let view: View = { width: 1, height: 1, imageWidth: 1, imageHeight: 1 };
  let natural = { width: 1, height: 1 };
  let returnFocus: HTMLElement | null = null;
  const pointers = new Map<number, Point>();
  let gesture: { start: Point; zoom: ZoomState; distance: number; mid: Point } | null = null;
  let moved = false;
  // Whether the current press began on the image. Recorded at pointerdown, before any pointer
  // capture, because a captured pointer's click is dispatched to the stage, not the image.
  let pressOnImage = false;

  // Pointer position relative to the viewport centre; a missing coordinate means the centre.
  const centre = (clientX: number, clientY: number): Point => ({
    x: Number.isFinite(clientX) ? clientX - view.width / 2 : 0,
    y: Number.isFinite(clientY) ? clientY - view.height / 2 : 0,
  });
  const apply = (): void => {
    image.style.transform = toTransform(zoom);
    overlay.classList.toggle('is-zoomed', zoom.scale > 1.001);
  };
  const layout = (): void => {
    const width = win.innerWidth;
    const height = win.innerHeight;
    const fit = fitImage(natural.width, natural.height, width, height, width < 640 ? 12 : 56);
    view = { width, height, imageWidth: fit.width, imageHeight: fit.height };
    image.style.width = `${fit.width}px`;
    image.style.height = `${fit.height}px`;
    image.style.marginLeft = `${-fit.width / 2}px`;
    image.style.marginTop = `${-fit.height / 2}px`;
    zoom = panBy(zoom, 0, 0, view);
    apply();
  };
  const set = (next: ZoomState): void => {
    zoom = next;
    apply();
  };

  function open(trigger: HTMLAnchorElement): void {
    const thumb = trigger.querySelector('img');
    natural = {
      width: Number(trigger.dataset.width) || thumb?.naturalWidth || 1,
      height: Number(trigger.dataset.height) || thumb?.naturalHeight || 1,
    };
    returnFocus = trigger;
    image.src = trigger.href;
    image.alt = thumb?.alt ?? '';
    zoom = IDENTITY;
    overlay.hidden = false;
    doc.documentElement.classList.add('lightbox-open');
    layout();
    closeButton.focus();
    if (!prefersReducedMotion() && typeof image.animate === 'function' && thumb) {
      const rect = thumb.getBoundingClientRect();
      if (rect.width > 0) {
        const from = toTransform({
          scale: rect.width / view.imageWidth,
          x: rect.left + rect.width / 2 - view.width / 2,
          y: rect.top + rect.height / 2 - view.height / 2,
        });
        image.animate([{ transform: from }, { transform: toTransform(zoom) }], {
          duration: 380,
          easing: 'cubic-bezier(0.16, 1, 0.3, 1)',
        });
      }
    }
    requestFrame(() => overlay.classList.add('is-open'));
  }

  function requestFrame(callback: () => void): void {
    if (typeof win.requestAnimationFrame === 'function') win.requestAnimationFrame(callback);
    else callback();
  }

  function close(): void {
    if (overlay.hidden) return;
    overlay.classList.remove('is-open', 'is-zoomed', 'is-dragging');
    overlay.hidden = true;
    pointers.clear();
    gesture = null;
    pressOnImage = false;
    doc.documentElement.classList.remove('lightbox-open');
    image.removeAttribute('src');
    returnFocus?.focus();
    returnFocus = null;
  }

  for (const trigger of triggers) {
    trigger.addEventListener('click', (event) => {
      if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey) return;
      event.preventDefault();
      open(trigger);
    });
  }

  closeButton.addEventListener('click', (event) => {
    event.stopPropagation();
    close();
  });

  // A click that lands outside the image closes, unless it ended a drag.
  overlay.addEventListener('click', (event) => {
    if (moved) {
      moved = false;
      return;
    }
    if (event.target !== image && !pressOnImage) close();
    pressOnImage = false;
  });

  overlay.addEventListener('keydown', (event) => {
    const middle: Point = { x: 0, y: 0 };
    switch (event.key) {
      case 'Escape':
        event.preventDefault();
        close();
        return;
      case 'Tab': {
        // Two focus stops: the zoomable stage and the close button.
        event.preventDefault();
        (doc.activeElement === closeButton ? stage : closeButton).focus();
        return;
      }
      case '+':
      case '=':
        set(zoomAt(zoom, KEY_STEP, middle, view));
        break;
      case '-':
      case '_':
        set(zoomAt(zoom, 1 / KEY_STEP, middle, view));
        break;
      case '0':
        set(IDENTITY);
        break;
      case 'ArrowLeft':
        set(panBy(zoom, PAN_STEP, 0, view));
        break;
      case 'ArrowRight':
        set(panBy(zoom, -PAN_STEP, 0, view));
        break;
      case 'ArrowUp':
        set(panBy(zoom, 0, PAN_STEP, view));
        break;
      case 'ArrowDown':
        set(panBy(zoom, 0, -PAN_STEP, view));
        break;
      case ' ':
      case 'PageUp':
      case 'PageDown':
      case 'Home':
      case 'End':
        // The page behind the dialog must not scroll.
        break;
      default:
        return;
    }
    event.preventDefault();
  });

  overlay.addEventListener(
    'wheel',
    (event) => {
      event.preventDefault();
      const delta = event.deltaMode === 1 ? event.deltaY * 16 : event.deltaY;
      set(zoomAt(zoom, wheelFactor(delta), centre(event.clientX, event.clientY), view));
    },
    { passive: false },
  );

  stage.addEventListener('dblclick', (event) => {
    if (event.target !== image && !pressOnImage) return;
    const point = centre(event.clientX, event.clientY);
    set(zoom.scale > 1.001 ? IDENTITY : zoomTo(zoom, DOUBLE_CLICK_SCALE, point, view));
  });

  const distance = (a: Point, b: Point): number => Math.hypot(a.x - b.x, a.y - b.y) || 1;
  const startGesture = (): void => {
    const points = [...pointers.values()];
    const [a, b] = points;
    if (!a) {
      gesture = null;
      return;
    }
    gesture = {
      start: a,
      zoom,
      distance: b ? distance(a, b) : 1,
      mid: b ? { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 } : a,
    };
  };

  // Capture only once a drag or pinch starts, so plain clicks and taps keep their real target.
  const capture = (): void => {
    for (const id of pointers.keys()) {
      try {
        if (!stage.hasPointerCapture?.(id)) stage.setPointerCapture?.(id);
      } catch {
        // The pointer already ended; nothing to capture.
      }
    }
  };
  stage.addEventListener('pointerdown', (event) => {
    if (event.button !== 0 && event.pointerType === 'mouse') return;
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    if (pointers.size === 1) pressOnImage = event.target === image;
    moved = false;
    startGesture();
  });
  stage.addEventListener('pointermove', (event) => {
    if (!pointers.has(event.pointerId) || !gesture) return;
    pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
    const [a, b] = [...pointers.values()];
    if (!a) return;
    if (b) {
      const mid = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
      const scaled = zoomAt(
        gesture.zoom,
        distance(a, b) / gesture.distance,
        centre(gesture.mid.x, gesture.mid.y),
        view,
      );
      set(panBy(scaled, mid.x - gesture.mid.x, mid.y - gesture.mid.y, view));
      capture();
      moved = true;
    } else {
      const dx = a.x - gesture.start.x;
      const dy = a.y - gesture.start.y;
      if (!moved && Math.hypot(dx, dy) < DRAG_THRESHOLD) return;
      moved = true;
      capture();
      overlay.classList.add('is-dragging');
      set(panBy(gesture.zoom, dx, dy, view));
    }
  });
  const endPointer = (event: PointerEvent): void => {
    if (!pointers.delete(event.pointerId)) return;
    overlay.classList.remove('is-dragging');
    startGesture();
  };
  stage.addEventListener('pointerup', endPointer);
  stage.addEventListener('pointercancel', endPointer);

  win.addEventListener('resize', () => {
    if (!overlay.hidden) layout();
  });

  return {
    element: overlay,
    open,
    close,
    isOpen: () => !overlay.hidden,
    state: () => zoom,
  };
}
