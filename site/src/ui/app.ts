import { getAsylumDenial } from '../logic/asylum';
import { getLiveObservation } from '../logic/analysis';

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

function initTraces(root: HTMLElement): void {
  const controls = [...root.querySelectorAll<HTMLButtonElement>('[data-trace]')];
  if (controls.length === 0 || typeof Element.prototype.animate !== 'function') return;

  const preference = window.matchMedia('(prefers-reduced-motion: reduce)');
  const active = new Map<HTMLButtonElement, Animation[]>();
  const clear = (button: HTMLButtonElement): void => {
    for (const animation of active.get(button) ?? []) animation.cancel();
    active.delete(button);
    button.closest('.reference-flow, #roadmap')?.classList.remove('is-tracing');
  };
  const syncPreference = (): void => {
    if (preference.matches) {
      for (const button of controls) {
        const hadFocus = document.activeElement === button;
        clear(button);
        button.hidden = true;
        if (hadFocus) {
          const scope = button.closest('.reference-flow, #roadmap');
          const heading =
            scope?.querySelector('h2') ?? button.closest('section')?.querySelector('h2');
          heading?.setAttribute('tabindex', '-1');
          heading?.focus();
        }
      }
    } else {
      for (const button of controls) button.hidden = false;
    }
  };
  for (const button of controls) {
    button.addEventListener('click', () => {
      clear(button);
      const scope =
        button.dataset.trace === 'flow'
          ? button.closest('.reference-flow')
          : root.querySelector('#roadmap');
      if (!scope) return;
      scope.classList.add('is-tracing');
      const targets =
        button.dataset.trace === 'flow'
          ? [scope.querySelector<SVGCircleElement>('.flow-trace')]
          : [scope.querySelector<HTMLElement>('.milestone-trace')];
      const animations = targets.flatMap((target) => {
        if (!target) return [];
        // SVG transform distances are expressed in the viewBox coordinate space.
        const distance = button.dataset.trace === 'flow' ? 410 : 0;
        const animation = target.animate(
          button.dataset.trace === 'flow'
            ? [{ transform: 'translateX(0)' }, { transform: `translateX(${distance}px)` }]
            : [{ transform: 'scaleY(0)' }, { transform: 'scaleY(1)' }],
          {
            duration: 700,
            easing: 'ease-out',
            fill: 'both',
          },
        );
        animation.onfinish = () => animation.cancel();
        return [animation];
      });
      active.set(button, animations);
    });
  }
  preference.addEventListener('change', syncPreference);
  syncPreference();
}

export function initApp(root: HTMLElement): void {
  initDossier(root);
  initTraces(root);
  root.dataset.enhancementsReady = 'true';
}
