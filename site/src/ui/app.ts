import { getAsylumDenial } from '../logic/asylum';
import { getLiveObservation } from '../logic/analysis';
import { initLightbox } from './lightbox';

function initDossier(root: HTMLElement): void {
  const analysis = root.querySelector<HTMLElement>('#analysis-output');
  const button = root.querySelector<HTMLButtonElement>('#asylum-button');
  const output = root.querySelector<HTMLElement>('#asylum-output');
  if (!analysis || !button || !output) return;
  let clicks = 0;
  button.addEventListener('click', () => {
    output.textContent = getAsylumDenial(clicks++);
  });
  // The dossier joke changes only local text in response to local events.
  for (const event of ['mousemove', 'keydown', 'click']) {
    root.addEventListener(event, () => {
      analysis.textContent = getLiveObservation(event);
    });
  }
}

const reducedMotion = (): boolean => window.matchMedia('(prefers-reduced-motion: reduce)').matches;

/**
 * Scroll reveals. Browsers with CSS scroll-driven animations need nothing here.
 * Elsewhere an IntersectionObserver adds `is-in` once; `html.io-reveal` is only
 * set when the observer exists, so content is never hidden without scripts.
 */
function initReveal(): void {
  if (reducedMotion() || typeof IntersectionObserver !== 'function') return;
  // `.flow` always uses the observer: its reference pulses start only once it is seen.
  const scrollDriven = CSS.supports?.('animation-timeline: view()') ?? false;
  if (!scrollDriven) document.documentElement.classList.add('io-reveal');
  const selector = scrollDriven ? '.flow' : '.reveal, .stage, .milestone, .rail, .flow';
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        entry.target.classList.add('is-in');
        observer.unobserve(entry.target);
      }
    },
    { rootMargin: '0px 0px -10% 0px' },
  );
  for (const el of document.querySelectorAll(selector)) observer.observe(el);
}

/** The hero glow follows the pointer; only `transform` changes, once per frame. */
function initGlow(): void {
  const hero = document.querySelector<HTMLElement>('.hero');
  const glow = hero?.querySelector<HTMLElement>('.hero-glow');
  if (!hero || !glow || reducedMotion() || !window.matchMedia('(pointer: fine)').matches) return;
  let frame = 0;
  hero.addEventListener('pointermove', (event) => {
    if (frame) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      const rect = hero.getBoundingClientRect();
      glow.style.transform = `translate3d(${event.clientX - rect.left}px, ${event.clientY - rect.top}px, 0)`;
    });
  });
}

/** A copy button on each command block; the commands stay selectable without it. */
function initCopy(): void {
  if (!navigator.clipboard) return;
  for (const pre of document.querySelectorAll<HTMLPreElement>('.terminal pre')) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'copy-button';
    button.textContent = 'Copy';
    button.addEventListener('click', () => {
      navigator.clipboard.writeText(pre.textContent ?? '').then(
        () => {
          button.textContent = 'Copied';
          setTimeout(() => (button.textContent = 'Copy'), 1600);
        },
        () => (button.textContent = 'Select and copy'),
      );
    });
    const block = document.createElement('div');
    block.className = 'code-block';
    pre.before(block);
    block.append(pre, button);
  }
}

export function initApp(root: HTMLElement): void {
  const sleeve = root.querySelector<HTMLInputElement>('#sleeve-flip');
  const front = root.querySelector<HTMLElement>('.sleeve-front');
  const back = root.querySelector<HTMLElement>('.sleeve-back');
  if (sleeve && front && back) {
    const syncSleeve = (): void => {
      front.setAttribute('aria-hidden', String(sleeve.checked));
      back.setAttribute('aria-hidden', String(!sleeve.checked));
    };
    sleeve.addEventListener('change', syncSleeve);
    syncSleeve();
  }
  initDossier(root);
  initReveal();
  initGlow();
  initCopy();
  initLightbox(document);
  root.dataset.enhancementsReady = 'true';
}
