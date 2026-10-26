import { milestones, productGoals, projectUpdates, roadmapSnapshot } from '../data/roadmap';

/** Build-time rendering keeps the typed source authoritative, even without JavaScript. */
export function escapeHtml(value: string): string {
  return value.replace(/[&<>"']/g, (character) => {
    const entities: Record<string, string> = {
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      '"': '&quot;',
      "'": '&#39;',
    };
    return entities[character] ?? character;
  });
}

function link(label: string, href: string): string {
  return `<a class="source-link" href="${escapeHtml(href)}" rel="noopener noreferrer">${escapeHtml(label)}</a>`;
}

export function renderRoadmap(html: string): string {
  const fragments: Record<string, string> = {
    currentSummary: escapeHtml(roadmapSnapshot.currentSummary),
    currentFocus: escapeHtml(roadmapSnapshot.currentFocus),
    verifiedAt: escapeHtml(roadmapSnapshot.verifiedAt),
    qualificationSummary: escapeHtml(roadmapSnapshot.qualificationSummary),
    milestones: milestones
      .map(
        (milestone, index) => `
      <li class="milestone milestone-${milestone.status}">
        <div class="milestone-marker" aria-hidden="true"><span class="milestone-code">${escapeHtml(milestone.id)}</span></div>
        <article class="milestone-card" aria-labelledby="milestone-title-${index}">
          <div class="milestone-card-header">
            <h3 class="milestone-title" id="milestone-title-${index}">${escapeHtml(milestone.title)}</h3>
            <span class="status-chip">${escapeHtml(milestone.statusLabel)}</span>
          </div>
          <p class="milestone-goal">${escapeHtml(milestone.goal)}</p>
          <dl class="milestone-details">
            <dt class="detail-label">Progress</dt><dd class="detail-value">${escapeHtml(milestone.progress)}</dd>
            <dt class="detail-label">Next</dt><dd class="detail-value">${escapeHtml(milestone.next)}</dd>
          </dl>
          <div class="milestone-card-footer">${link(milestone.source.label, milestone.source.href)}</div>
        </article>
      </li>`,
      )
      .join(''),
    updates: projectUpdates
      .map(
        (update) => `
      <li class="update-item">
        <time datetime="${escapeHtml(update.date)}">${escapeHtml(update.date)}</time>
        <div class="update-copy">
          <h3 class="update-title">${escapeHtml(update.title)}</h3>
          <p class="update-category">${escapeHtml(update.category.replaceAll(' · ', ': '))}</p>
          <p class="update-detail">${escapeHtml(update.detail)}</p>
          ${link(update.source.label, update.source.href)}
        </div>
      </li>`,
      )
      .join(''),
    goals: productGoals.map((goal) => `<li>${escapeHtml(goal)}</li>`).join(''),
  };
  return html.replace(/<!-- roadmap:(\w+) -->/g, (_, key: string) => {
    const fragment = fragments[key];
    if (fragment === undefined) throw new Error(`Unknown roadmap fragment: ${key}`);
    return fragment;
  });
}
