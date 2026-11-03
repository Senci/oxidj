/**
 * Build-time SVG art for the home hero: a deterministic, synthetic 3-band
 * waveform over a 4/4 beat grid. It is illustration only (no real audio),
 * drawn in the same low/mid/high colour roles as the prototype's 3-band view.
 */

export interface WaveArt {
  readonly width: number;
  readonly height: number;
  readonly bands: { readonly low: string; readonly mid: string; readonly high: string };
  readonly beats: readonly { readonly x: number; readonly bar: number | null }[];
}

/** Small deterministic PRNG (mulberry32) so the build output is stable. */
export function prng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Closed, vertically mirrored envelope path around the centre line. */
export function envelopePath(amplitudes: readonly number[], width: number, height: number): string {
  if (amplitudes.length < 2) return '';
  const mid = height / 2;
  const step = width / (amplitudes.length - 1);
  const fmt = (n: number): string => (Math.round(n * 10) / 10).toString();
  const top = amplitudes.map((a, i) => `${fmt(i * step)} ${fmt(mid - a * mid)}`);
  const bottom = amplitudes.map((a, i) => `${fmt(i * step)} ${fmt(mid + a * mid)}`).reverse();
  return `M${top.join('L')}L${bottom.join('L')}Z`;
}

export function waveArt(options: { beats?: number; points?: number; seed?: number } = {}): WaveArt {
  const beats = options.beats ?? 32;
  const points = options.points ?? 320;
  const random = prng(options.seed ?? 42);
  const width = 1200;
  const height = 240;
  const low: number[] = [];
  const mid: number[] = [];
  const high: number[] = [];
  for (let i = 0; i < points; i++) {
    const t = (i / (points - 1)) * beats; // position in beats
    const beat = Math.floor(t);
    const phase = t - beat;
    const intro = Math.min(1, t / 3); // fade in over the first bar
    const kick = Math.exp(-phase * 5.5);
    const snare = beat % 4 === 1 || beat % 4 === 3 ? Math.exp(-phase * 7) : 0;
    const offbeat = Math.abs(phase - 0.5);
    const hat = Math.exp(-offbeat * 18);
    const breakdown = beat >= 16 && beat < 20 ? 0.35 : 1; // a short breakdown in bar 5
    low.push(intro * breakdown * (0.18 + 0.8 * kick) * (0.92 + random() * 0.08));
    mid.push(intro * (0.16 + 0.5 * snare + 0.12 * random()) * (beat >= 16 && beat < 20 ? 0.8 : 1));
    high.push(intro * (0.05 + 0.2 * hat + 0.07 * random()));
  }
  return {
    width,
    height,
    bands: {
      low: envelopePath(low, width, height),
      mid: envelopePath(mid, width, height),
      high: envelopePath(high, width, height),
    },
    beats: Array.from({ length: beats + 1 }, (_, b) => ({
      x: Math.round((b / beats) * width * 10) / 10,
      bar: b % 4 === 0 && b < beats ? b / 4 + 1 : null,
    })),
  };
}

/** Inline SVG markup for the hero. Decorative: `aria-hidden`. */
export function renderWaveSvg(art: WaveArt = waveArt()): string {
  const grid = art.beats
    .map(
      (b, i) =>
        `<line class="${b.bar === null ? 'beat' : 'bar'}" style="--i:${i}" x1="${b.x}" x2="${b.x}" y1="6" y2="${art.height - 6}" />`,
    )
    .join('');
  const labels = art.beats
    .filter((b) => b.bar !== null)
    .map(
      (b) =>
        `<span style="left:${Math.round((b.x / art.width) * 10000) / 100}%">Bar ${b.bar}</span>`,
    )
    .join('');
  // Labels are HTML so the stretched (preserveAspectRatio="none") SVG never distorts text.
  return `<div class="wave" aria-hidden="true"><svg class="wave-art" viewBox="0 0 ${art.width} ${art.height}" preserveAspectRatio="none" focusable="false"><g class="wave-grid">${grid}</g><g class="wave-bands"><path class="band band-low" d="${art.bands.low}" /><path class="band band-mid" d="${art.bands.mid}" /><path class="band band-high" d="${art.bands.high}" /></g><rect class="wave-unplayed" x="0" y="0" width="${art.width}" height="${art.height}" /><line class="wave-playhead" x1="0" x2="0" y1="0" y2="${art.height}" /></svg><div class="wave-labels">${labels}</div></div>`;
}
