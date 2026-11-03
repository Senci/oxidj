// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { initLightbox, type Lightbox } from '../ui/lightbox';

function markup(): string {
  return `<main><button id="before">Before</button>
    <a class="shot-link" href="./images/app-overview.webp" data-lightbox data-width="1440" data-height="793"><img src="./images/app-overview-720.webp" width="1440" height="793" alt="Library window" /></a>
    <a class="shot-link" href="./images/waveform-3band.webp" data-lightbox data-width="359" data-height="405"><img src="./images/waveform-3band.webp" width="359" height="405" alt="3-band waveform" /></a>
  </main>`;
}

const key = (target: Element, k: string): KeyboardEvent => {
  const event = new KeyboardEvent('keydown', { key: k, bubbles: true, cancelable: true });
  target.dispatchEvent(event);
  return event;
};
const pointer = (target: Element, type: string, x: number, y: number, id = 1): void => {
  target.dispatchEvent(
    new PointerEvent(type, {
      pointerId: id,
      clientX: x,
      clientY: y,
      button: 0,
      pointerType: 'mouse',
      bubbles: true,
    }),
  );
};

describe('screenshot lightbox', () => {
  let box: Lightbox;
  let triggers: HTMLAnchorElement[];

  beforeEach(() => {
    document.body.innerHTML = markup();
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1440 });
    Object.defineProperty(window, 'innerHeight', { configurable: true, value: 900 });
    const created = initLightbox(document, () => true);
    if (!created) throw new Error('lightbox not created');
    box = created;
    triggers = [...document.querySelectorAll<HTMLAnchorElement>('a[data-lightbox]')];
  });
  afterEach(() => {
    document.body.innerHTML = '';
    document.documentElement.className = '';
  });

  it('returns null when a page has no zoomable images', () => {
    document.body.innerHTML = '<p>No images</p>';
    expect(initLightbox(document)).toBeNull();
  });

  it('opens from a click as a labelled modal dialog with the full-resolution image', () => {
    const event = new MouseEvent('click', { bubbles: true, cancelable: true, button: 0 });
    triggers[0]?.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    expect(box.isOpen()).toBe(true);
    expect(box.element.getAttribute('role')).toBe('dialog');
    expect(box.element.getAttribute('aria-modal')).toBe('true');
    expect(box.element.getAttribute('aria-label')).toBe('Screenshot viewer');
    const image = box.element.querySelector<HTMLImageElement>('.lightbox-image');
    expect(image?.getAttribute('src')).toMatch(/\/images\/app-overview\.webp$/);
    expect(image?.alt).toBe('Library window');
    expect(image?.style.width).toBe('1328px');
    expect(document.documentElement.classList.contains('lightbox-open')).toBe(true);
    expect(document.activeElement).toBe(box.element.querySelector('.lightbox-close'));
  });

  it('closes with the X button and returns focus to the trigger', () => {
    triggers[1]?.focus();
    triggers[1]?.click();
    box.element.querySelector<HTMLButtonElement>('.lightbox-close')?.click();
    expect(box.isOpen()).toBe(false);
    expect(document.activeElement).toBe(triggers[1]);
    expect(document.documentElement.classList.contains('lightbox-open')).toBe(false);
  });

  it('closes with Escape', () => {
    triggers[0]?.click();
    const event = key(box.element, 'Escape');
    expect(event.defaultPrevented).toBe(true);
    expect(box.isOpen()).toBe(false);
    expect(document.activeElement).toBe(triggers[0]);
  });

  it('closes on a click outside the image but not on the image itself', () => {
    triggers[0]?.click();
    const image = box.element.querySelector('.lightbox-image');
    image?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(box.isOpen()).toBe(true);
    box.element
      .querySelector('.lightbox-stage')
      ?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(box.isOpen()).toBe(false);
  });

  it('keeps Tab focus inside the dialog', () => {
    triggers[0]?.click();
    const close = box.element.querySelector('.lightbox-close');
    const stage = box.element.querySelector('.lightbox-stage');
    expect(key(box.element, 'Tab').defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(stage);
    key(box.element, 'Tab');
    expect(document.activeElement).toBe(close);
  });

  it('zooms with + and - within the clamp, resets with 0 and pans with the arrow keys', () => {
    triggers[0]?.click();
    key(box.element, '-');
    expect(box.state().scale).toBe(1);
    for (let i = 0; i < 30; i++) key(box.element, '+');
    expect(box.state().scale).toBe(4);
    key(box.element, 'ArrowRight');
    expect(box.state().x).toBe(-64);
    key(box.element, 'ArrowDown');
    expect(box.state().y).toBe(-64);
    key(box.element, '0');
    expect(box.state()).toEqual({ scale: 1, x: 0, y: 0 });
    const image = box.element.querySelector<HTMLImageElement>('.lightbox-image');
    expect(image?.style.transform).toBe('translate3d(0.00px, 0.00px, 0) scale(1.0000)');
  });

  it('zooms at the cursor with the wheel and toggles with double-click', () => {
    triggers[0]?.click();
    const wheel = new WheelEvent('wheel', { deltaY: -400, bubbles: true, cancelable: true });
    // happy-dom leaves pointer coordinates off WheelEvent; real browsers always set them.
    Object.defineProperties(wheel, { clientX: { value: 1020 }, clientY: { value: 450 } });
    box.element.dispatchEvent(wheel);
    expect(wheel.defaultPrevented).toBe(true);
    const zoomed = box.state();
    expect(zoomed.scale).toBeGreaterThan(1.5);
    // Zooming at a point right of centre moves the image left to keep that point fixed.
    expect(zoomed.x).toBeLessThan(0);
    const image = box.element.querySelector('.lightbox-image');
    image?.dispatchEvent(new MouseEvent('dblclick', { clientX: 720, clientY: 450, bubbles: true }));
    expect(box.state().scale).toBe(1);
    image?.dispatchEvent(new MouseEvent('dblclick', { clientX: 720, clientY: 450, bubbles: true }));
    expect(box.state().scale).toBe(2.5);
  });

  it('pans by dragging and a drag never closes the dialog', () => {
    triggers[0]?.click();
    key(box.element, '+');
    key(box.element, '+');
    const stage = box.element.querySelector('.lightbox-stage');
    if (!stage) throw new Error('no stage');
    pointer(stage, 'pointerdown', 700, 450);
    pointer(stage, 'pointermove', 640, 430);
    expect(box.element.classList.contains('is-dragging')).toBe(true);
    pointer(stage, 'pointerup', 640, 430);
    expect(box.state().x).toBe(-60);
    expect(box.state().y).toBe(-20);
    stage.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(box.isOpen()).toBe(true);
  });

  it('pinch-zooms with two pointers', () => {
    triggers[0]?.click();
    const stage = box.element.querySelector('.lightbox-stage');
    if (!stage) throw new Error('no stage');
    pointer(stage, 'pointerdown', 700, 450, 1);
    pointer(stage, 'pointerdown', 740, 450, 2);
    pointer(stage, 'pointermove', 780, 450, 2);
    expect(box.state().scale).toBeCloseTo(2, 5);
  });

  it('resets zoom each time it opens', () => {
    triggers[0]?.click();
    key(box.element, '+');
    key(box.element, 'Escape');
    triggers[1]?.click();
    expect(box.state()).toEqual({ scale: 1, x: 0, y: 0 });
    const image = box.element.querySelector<HTMLImageElement>('.lightbox-image');
    expect(image?.getAttribute('src')).toMatch(/\/images\/waveform-3band\.webp$/);
  });

  it('stops page-scrolling keys from scrolling the page behind the dialog', () => {
    triggers[0]?.click();
    for (const k of [' ', 'PageDown', 'End'])
      expect(key(box.element, k).defaultPrevented).toBe(true);
    expect(box.isOpen()).toBe(true);
  });

  it('leaves modified clicks to the browser', () => {
    const event = new MouseEvent('click', { bubbles: true, cancelable: true, ctrlKey: true });
    triggers[0]?.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(false);
    expect(box.isOpen()).toBe(false);
  });
});
