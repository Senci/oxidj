/**
 * Redaction helper for the dossier.
 *
 * Replaces the trailing portion of a string with full-width block characters
 * (U+2588), keeping the total length stable so surrounding layout is preserved.
 * `keep` controls how many leading characters remain visible.
 */

const REDACTION_CHAR = '█';

export function redact(text: string, keep = 0): string {
  if (text.length === 0) {
    return '';
  }
  const keepCount = Math.min(Math.max(0, keep), text.length);
  const visible = text.slice(0, keepCount);
  const masked = REDACTION_CHAR.repeat(text.length - keepCount);
  return visible + masked;
}
