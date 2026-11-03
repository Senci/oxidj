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
    // Every milestone sits on one timeline with its status label always shown, so a
    // started later milestone is never mislabelled by its position in the list.
    milestones: milestones
      .map(
        (m, i) =>
          `<li id="${m.id}" class="milestone milestone-${m.status}" style="--i:${i}"><span class="milestone-node" aria-hidden="true"></span><span class="milestone-code">${m.id.toUpperCase()}</span><article><h3>${escapeHtml(m.title)}</h3><p class="status-label status-${m.status}">${escapeHtml(m.statusLabel)}</p><p>${escapeHtml(m.detail)}</p>${link(m.source.label, m.source.href)}</article></li>`,
      )
      .join(''),
    rail: milestones
      .map(
        (m, i) =>
          `<li class="rail-stop rail-${m.status}" style="--i:${i}"><a href="./roadmap.html#${m.id}"><span class="rail-code">${m.id.toUpperCase()}</span><span class="rail-title">${escapeHtml(m.title)}</span><span class="rail-status">${escapeHtml(m.statusLabel)}</span></a></li>`,
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
