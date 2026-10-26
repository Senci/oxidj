/**
 * Live-analysis observations.
 *
 * The "LIVE ANALYSIS" panel reacts to visitor behaviour (mouse movement,
 * typing, clicks) with deadpan observations. The mapping is a pure function
 * of the event name so the teaser logic stays unit-testable; the UI layer
 * decides which DOM events map to which observation keys.
 */

export const DEFAULT_OBSERVATION = 'OBSERVATION PENDING. THE ARCHIVE LISTENS.';

export function getLiveObservation(event: string): string {
  switch (event) {
    case 'mousemove':
      return 'Cursor detected. Analyzing.';
    case 'keydown':
      return 'SUBJECT IS HESITATING';
    case 'click':
      return 'INTEREST LOGGED';
    case 'idle':
      return 'SILENCE. THE ARCHIVE LISTENS.';
    default:
      return DEFAULT_OBSERVATION;
  }
}
