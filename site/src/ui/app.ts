import { getAsylumDenial } from '../logic/asylum';
import { getLiveObservation } from '../logic/analysis';

export function initApp(root: HTMLElement): void {
  const analysis = root.querySelector<HTMLElement>('#analysis-output');
  const button = root.querySelector<HTMLButtonElement>('#asylum-button');
  const output = root.querySelector<HTMLElement>('#asylum-output');
  if (!analysis || !button || !output) throw new Error('Required DOM elements are missing');
  let clicks = 0;
  button.addEventListener('click', () => {
    output.textContent = getAsylumDenial(clicks++);
  });
  // No tracking: the joke only changes local text in response to events.
  for (const event of ['mousemove', 'keydown', 'click']) {
    root.addEventListener(event, () => {
      analysis.textContent = getLiveObservation(event);
    });
  }
}
