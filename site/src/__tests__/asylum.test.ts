import { describe, expect, it } from 'vitest';

import { ASYLUM_DENIALS, getAsylumDenial, totalDenials } from '../logic/asylum';

describe('getAsylumDenial', () => {
  it('returns the first denial for index 0', () => {
    expect(getAsylumDenial(0)).toBe(
      'ASYLUM DENIED. Your library has already been reviewed. It has notes.',
    );
  });

  it('returns the second denial for index 1', () => {
    expect(getAsylumDenial(1)).toBe(
      'ASYLUM DENIED. We see you. Your playlists are not convincing.',
    );
  });

  it('returns the third denial for index 2', () => {
    expect(getAsylumDenial(2)).toBe(
      'ASYLUM DENIED. Your beat-match history suggests premeditation.',
    );
  });

  it('wraps to the first denial after the last (repeated clicks)', () => {
    const n = totalDenials();
    expect(getAsylumDenial(n)).toBe(getAsylumDenial(0));
    expect(getAsylumDenial(n + 1)).toBe(getAsylumDenial(1));
  });

  it('cycles across repeated clicks', () => {
    const n = totalDenials();
    const seen: string[] = [];
    for (let i = 0; i < n * 2; i++) {
      seen.push(getAsylumDenial(i));
    }
    expect(seen).toHaveLength(n * 2);
    expect(seen[0]).toBe(seen[n]);
  });

  it('handles negative indices gracefully', () => {
    expect(getAsylumDenial(-1)).toBe(getAsylumDenial(totalDenials() - 1));
  });

  it('always returns a non-empty denial with the correct prefix', () => {
    for (let i = 0; i < 25; i++) {
      const message = getAsylumDenial(i);
      expect(message.length).toBeGreaterThan(0);
      expect(message.startsWith('ASYLUM DENIED')).toBe(true);
    }
  });

  it('denials are all distinct', () => {
    expect(new Set(ASYLUM_DENIALS).size).toBe(ASYLUM_DENIALS.length);
  });
});
