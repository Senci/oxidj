/**
 * Asylum-denial responses.
 *
 * The "REQUEST ASYLUM" button cycles through these deadpan rejections.
 * `getAsylumDenial` is a pure function of the click counter, so the UI just
 * increments a counter and the cycling logic stays unit-testable.
 */

export const ASYLUM_DENIALS: readonly string[] = [
  'ASYLUM DENIED. Your library has already been reviewed. It has notes.',
  'ASYLUM DENIED. We see you. Your playlists are not convincing.',
  'ASYLUM DENIED. Your beat-match history suggests premeditation.',
];

export function getAsylumDenial(clickIndex: number): string {
  const n = ASYLUM_DENIALS.length;
  const idx = ((clickIndex % n) + n) % n;
  const message = ASYLUM_DENIALS[idx];
  if (message === undefined) {
    return 'ASYLUM DENIED.';
  }
  return message;
}

export function totalDenials(): number {
  return ASYLUM_DENIALS.length;
}
