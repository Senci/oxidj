/**
 * Evidence-log generator.
 *
 * The evidence log is the timestamped, deadpan record of the subject's
 * (the visitor's) behaviour. The content is fixed, but the generator and
 * its helpers are pure functions so the teaser logic stays unit-testable.
 */

export interface EvidenceEntry {
  /** Timestamp in 24-hour "HH:MM" format. */
  readonly time: string;
  /** Deadpan observation text. */
  readonly text: string;
}

export const EVIDENCE_ENTRIES: readonly EvidenceEntry[] = [
  { time: '03:12', text: 'Subject attempted a crossfade. Off-beat. Noted.' },
  { time: '03:47', text: 'Subject skipped a track at 0:04. Understandable. Noted.' },
  { time: '04:15', text: 'Subject claimed to "know what they are doing." Flagged for review.' },
  { time: '04:33', text: 'Subject hovered over a button without pressing it. Pathetic. Noted.' },
  { time: '05:01', text: 'Subject asked for asylum. Request archived for future amusement.' },
];

/**
 * Format an entry for display as a single line.
 */
export function formatEvidenceEntry(entry: EvidenceEntry): string {
  return `${entry.time} — ${entry.text}`;
}

/**
 * Return up to `limit` evidence entries. With no limit, returns every entry.
 * A limit of zero or less returns an empty list. The returned array is a
 * copy; mutating it never affects the source log.
 */
export function generateEvidenceLog(limit?: number): EvidenceEntry[] {
  if (limit === undefined) {
    return EVIDENCE_ENTRIES.slice();
  }
  if (limit <= 0) {
    return [];
  }
  return EVIDENCE_ENTRIES.slice(0, limit);
}

/**
 * Structural type guard for an {@link EvidenceEntry}.
 */
export function isValidEvidenceEntry(value: unknown): value is EvidenceEntry {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const record = value as Record<string, unknown>;
  const time = record['time'];
  const text = record['text'];
  return typeof time === 'string' && typeof text === 'string' && time.length > 0 && text.length > 0;
}
