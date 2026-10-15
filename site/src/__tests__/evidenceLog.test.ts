import { describe, expect, it } from 'vitest';

import {
  EVIDENCE_ENTRIES,
  formatEvidenceEntry,
  generateEvidenceLog,
  isValidEvidenceEntry,
} from '../logic/evidenceLog';

describe('generateEvidenceLog', () => {
  it('returns all entries when no limit is given', () => {
    expect(generateEvidenceLog()).toEqual(EVIDENCE_ENTRIES);
  });

  it('returns an empty array for a limit of 0', () => {
    expect(generateEvidenceLog(0)).toEqual([]);
  });

  it('returns an empty array for a negative limit', () => {
    expect(generateEvidenceLog(-3)).toEqual([]);
  });

  it('returns up to the requested number of entries', () => {
    const log = generateEvidenceLog(2);
    expect(log).toHaveLength(2);
    expect(log[0]?.time).toBe('03:12');
  });

  it('returns all available entries when the limit exceeds the log length', () => {
    expect(generateEvidenceLog(1000)).toHaveLength(EVIDENCE_ENTRIES.length);
  });

  it('returns a copy that does not mutate the source log', () => {
    const copy = generateEvidenceLog();
    copy[0] = { time: '99:99', text: 'tampered' };
    expect(EVIDENCE_ENTRIES[0]?.time).toBe('03:12');
    expect(EVIDENCE_ENTRIES[0]?.text).not.toBe('tampered');
  });

  it('every entry has a valid HH:MM timestamp and non-empty text', () => {
    for (const entry of generateEvidenceLog()) {
      expect(entry.time).toMatch(/^\d{2}:\d{2}$/);
      expect(entry.text.length).toBeGreaterThan(0);
    }
  });
});

describe('formatEvidenceEntry', () => {
  it('formats an entry as "time — text"', () => {
    expect(formatEvidenceEntry({ time: '03:12', text: 'hello' })).toBe('03:12 — hello');
  });

  it('handles empty text', () => {
    expect(formatEvidenceEntry({ time: '00:00', text: '' })).toBe('00:00 — ');
  });
});

describe('isValidEvidenceEntry', () => {
  it('accepts a well-formed entry', () => {
    expect(isValidEvidenceEntry({ time: '03:12', text: 'ok' })).toBe(true);
  });

  it('rejects non-object values', () => {
    expect(isValidEvidenceEntry('nope')).toBe(false);
    expect(isValidEvidenceEntry(42)).toBe(false);
    expect(isValidEvidenceEntry(null)).toBe(false);
    expect(isValidEvidenceEntry(undefined)).toBe(false);
  });

  it('rejects entries with missing or empty fields', () => {
    expect(isValidEvidenceEntry({})).toBe(false);
    expect(isValidEvidenceEntry({ time: '03:12' })).toBe(false);
    expect(isValidEvidenceEntry({ text: 'ok' })).toBe(false);
    expect(isValidEvidenceEntry({ time: '', text: 'ok' })).toBe(false);
    expect(isValidEvidenceEntry({ time: '03:12', text: '' })).toBe(false);
  });

  it('rejects entries with non-string fields', () => {
    expect(isValidEvidenceEntry({ time: 3, text: 'ok' })).toBe(false);
    expect(isValidEvidenceEntry({ time: '03:12', text: 5 })).toBe(false);
  });
});
