import { describe, expect, it } from 'vitest';

import { DEFAULT_OBSERVATION, getLiveObservation } from '../logic/analysis';

describe('getLiveObservation', () => {
  it('returns a cursor-detected observation for mousemove', () => {
    expect(getLiveObservation('mousemove')).toBe('CURSOR DETECTED — ANALYZING…');
  });

  it('returns a hesitation observation for keydown', () => {
    expect(getLiveObservation('keydown')).toBe('SUBJECT IS HESITATING');
  });

  it('returns an interest-logged observation for click', () => {
    expect(getLiveObservation('click')).toBe('INTEREST LOGGED');
  });

  it('returns the silence observation for idle', () => {
    expect(getLiveObservation('idle')).toBe('SILENCE. THE ARCHIVE LISTENS.');
  });

  it('returns the default observation for empty input', () => {
    expect(getLiveObservation('')).toBe(DEFAULT_OBSERVATION);
  });

  it('returns the default observation for unknown events', () => {
    expect(getLiveObservation('dblclick')).toBe(DEFAULT_OBSERVATION);
    expect(getLiveObservation('scroll')).toBe(DEFAULT_OBSERVATION);
    expect(getLiveObservation('touchstart')).toBe(DEFAULT_OBSERVATION);
  });

  it('is case-sensitive and treats mixed-case events as unknown', () => {
    expect(getLiveObservation('MouseMove')).toBe(DEFAULT_OBSERVATION);
    expect(getLiveObservation('CLICK')).toBe(DEFAULT_OBSERVATION);
  });

  it('never returns an empty string', () => {
    for (const event of ['mousemove', 'keydown', 'click', 'idle', '', 'unknown', 'xyz']) {
      expect(getLiveObservation(event).length).toBeGreaterThan(0);
    }
  });
});
