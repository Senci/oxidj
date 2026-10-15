import { getAsylumDenial } from '../logic/asylum';
import { generateEvidenceLog } from '../logic/evidenceLog';
import { getLiveObservation } from '../logic/analysis';
import { redact } from '../logic/redaction';

interface DossierField {
  label: string;
  value: string;
  redact?: boolean;
  keep?: number;
}

const DOSSIER_FIELDS: readonly DossierField[] = [
  { label: 'TRACKS CATALOGUED', value: '0 — collection not yet surrendered.' },
  { label: 'TRANSITIONS JUDGED', value: 'all of them.' },
  { label: 'VERDICT', value: 'pending. Subject remains at large.' },
  { label: 'KNOWN ASSOCIATES', value: 'a metadata schema that does not trust anyone.' },
  { label: 'SUBJECT NAME', value: 'DJ Unknown', redact: true, keep: 0 },
];

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props?: {
    className?: string;
    textContent?: string;
    attrs?: Record<string, string>;
  },
  children: (Node | string)[] = [],
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (props?.className) {
    node.className = props.className;
  }
  if (props?.textContent !== undefined) {
    node.textContent = props.textContent;
  }
  if (props?.attrs) {
    for (const [k, v] of Object.entries(props.attrs)) {
      node.setAttribute(k, v);
    }
  }
  for (const child of children) {
    node.append(child);
  }
  return node;
}

function renderDossier(dl: HTMLDListElement): void {
  for (const field of DOSSIER_FIELDS) {
    const dt = el('dt', { className: 'field-label', textContent: field.label });
    const value = field.redact === true ? redact(field.value, field.keep ?? 0) : field.value;
    const dd = el('dd', { className: 'field-value', textContent: value });
    dl.append(dt, dd);
  }
}

function renderEvidenceLog(ol: HTMLOListElement): void {
  for (const entry of generateEvidenceLog()) {
    const li = el('li', { className: 'evidence-entry' });
    const time = el('span', { className: 'evidence-time', textContent: entry.time });
    const text = el('span', { className: 'evidence-text', textContent: entry.text });
    li.append(time, text);
    ol.append(li);
  }
}

function setupLiveAnalysis(output: HTMLElement): void {
  let last = '';
  const report = (event: string): void => {
    const observation = getLiveObservation(event);
    if (observation !== last) {
      last = observation;
      output.textContent = observation;
    }
  };
  window.addEventListener('mousemove', () => report('mousemove'));
  window.addEventListener('keydown', () => report('keydown'));
  document.addEventListener('click', () => report('click'));
}

function setupAsylum(button: HTMLButtonElement, output: HTMLElement): void {
  let clicks = 0;
  button.addEventListener('click', () => {
    output.textContent = getAsylumDenial(clicks);
    clicks += 1;
  });
}

export function initApp(root: HTMLElement): void {
  const dossier = root.querySelector<HTMLDListElement>('#dossier-fields');
  const evidence = root.querySelector<HTMLOListElement>('#evidence-log');
  const analysisOutput = root.querySelector<HTMLElement>('#analysis-output');
  const asylumButton = root.querySelector<HTMLButtonElement>('#asylum-button');
  const asylumOutput = root.querySelector<HTMLElement>('#asylum-output');

  if (
    dossier === null ||
    evidence === null ||
    analysisOutput === null ||
    asylumButton === null ||
    asylumOutput === null
  ) {
    throw new Error('Required DOM elements are missing');
  }

  renderDossier(dossier);
  renderEvidenceLog(evidence);
  setupLiveAnalysis(analysisOutput);
  setupAsylum(asylumButton, asylumOutput);
}
