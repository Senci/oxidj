import { milestones, productGoals, projectUpdates, roadmapSnapshot } from './data/roadmap';
import './styles/roadmap.css';

function requiredElement<T extends HTMLElement>(selector: string): T {
  const element = document.querySelector<T>(selector);
  if (element === null) {
    throw new Error(`Required roadmap element not found: ${selector}`);
  }
  return element;
}

function text<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, value: string) {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = value;
  return element;
}

function link(label: string, href: string): HTMLAnchorElement {
  const anchor = document.createElement('a');
  anchor.className = 'source-link';
  anchor.href = href;
  anchor.textContent = label;
  anchor.rel = 'noopener noreferrer';
  anchor.append(document.createTextNode(' ↗'));
  return anchor;
}

function renderMilestones(): void {
  const list = requiredElement<HTMLOListElement>('#milestone-list');
  milestones.forEach((milestone, index) => {
    const item = document.createElement('li');
    item.className = `milestone milestone-${milestone.status}`;

    const marker = document.createElement('div');
    marker.className = 'milestone-marker';
    marker.setAttribute('aria-hidden', 'true');
    marker.append(text('span', 'milestone-code', milestone.id));

    const card = document.createElement('article');
    card.className = 'milestone-card';
    card.setAttribute('aria-labelledby', `milestone-title-${index}`);

    const header = document.createElement('div');
    header.className = 'milestone-card-header';
    const heading = text('h3', 'milestone-title', milestone.title);
    heading.id = `milestone-title-${index}`;
    const status = text('span', 'status-chip', milestone.statusLabel);
    header.append(heading, status);

    const goal = document.createElement('p');
    goal.className = 'milestone-goal';
    goal.textContent = milestone.goal;

    const details = document.createElement('dl');
    details.className = 'milestone-details';
    for (const [label, value] of [
      ['Progress', milestone.progress],
      ['Next', milestone.next],
    ] as const) {
      const term = text('dt', 'detail-label', label);
      const description = text('dd', 'detail-value', value);
      details.append(term, description);
    }

    const footer = document.createElement('div');
    footer.className = 'milestone-card-footer';
    footer.append(link(milestone.source.label, milestone.source.href));
    card.append(header, goal, details, footer);
    item.append(marker, card);
    list.append(item);
  });
}

function renderUpdates(): void {
  const list = requiredElement<HTMLOListElement>('#update-list');
  for (const update of projectUpdates) {
    const item = document.createElement('li');
    item.className = 'update-item';
    const time = document.createElement('time');
    time.dateTime = update.date;
    time.textContent = update.date;
    const copy = document.createElement('div');
    copy.className = 'update-copy';
    copy.append(
      text('span', 'update-category', update.category),
      text('h3', 'update-title', update.title),
      text('p', 'update-detail', update.detail),
      link(update.source.label, update.source.href),
    );
    item.append(time, copy);
    list.append(item);
  }
}

function renderGoals(): void {
  const list = requiredElement<HTMLUListElement>('#goal-list');
  productGoals.forEach((goal, index) => {
    const item = document.createElement('li');
    item.append(text('span', 'goal-index', `0${index + 1}`), text('span', 'goal-copy', goal));
    list.append(item);
  });
}

function renderSummary(): void {
  requiredElement<HTMLElement>('#current-summary').textContent = roadmapSnapshot.currentSummary;
  requiredElement<HTMLElement>('#current-focus').textContent = roadmapSnapshot.currentFocus;
  requiredElement<HTMLElement>('#snapshot-date').textContent = roadmapSnapshot.verifiedAt;
  requiredElement<HTMLElement>('#qualification-summary').textContent =
    roadmapSnapshot.qualificationSummary;
  requiredElement<HTMLAnchorElement>('#qualification-source').href =
    roadmapSnapshot.qualificationSource;
  requiredElement<HTMLAnchorElement>('#repo-link').href = roadmapSnapshot.repo;
}

renderSummary();
renderMilestones();
renderUpdates();
renderGoals();
