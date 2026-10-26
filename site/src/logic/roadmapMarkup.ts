import { milestones, projectUpdates, roadmapSnapshot } from '../data/roadmap';

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
  return `<a class="source-link" href="${escapeHtml(href)}">${escapeHtml(label)}</a>`;
}
export function renderRoadmap(html: string): string {
  const fragments: Record<string, string> = {
    currentSummary: escapeHtml(roadmapSnapshot.currentSummary),
    milestones: milestones
      .slice(0, 5)
      .map(
        (m) =>
          `<li id="${m.id}" class="milestone milestone-${m.status}"><span class="milestone-code">${m.id.toUpperCase()}</span><article><h3>${escapeHtml(m.title)}</h3><p class="status-label">${escapeHtml(m.statusLabel)}</p><p>${escapeHtml(m.detail)}</p>${link(m.source.label, m.source.href)}</article></li>`,
      )
      .join(''),
    later: milestones
      .slice(5)
      .map(
        (m) =>
          `<li id="${m.id}"><strong>${m.id.toUpperCase()}: ${escapeHtml(m.title)}</strong> ${escapeHtml(m.detail)}</li>`,
      )
      .join(''),
    updates: projectUpdates
      .map(
        (u) =>
          `<li><time datetime="${u.date}">${u.date}</time><div><h3>${escapeHtml(u.title)}</h3><p>${escapeHtml(u.detail)}</p>${link(u.source.label, u.source.href)}</div></li>`,
      )
      .join(''),
  };
  return html.replace(/<!-- roadmap:(\w+) -->/g, (_, key: string) => {
    if (fragments[key] === undefined) throw new Error(`Unknown roadmap fragment: ${key}`);
    return fragments[key];
  });
}
