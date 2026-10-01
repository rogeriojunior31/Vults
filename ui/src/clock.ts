// One clock for every animation, so the lab can freeze time for visual tests (`?t=<ms>`).

let frozen: number | null = null;

export const Clock = {
  now(): number {
    return frozen ?? performance.now();
  },
  /** Stops time at `ms` (visual tests); `null` lets it run again. */
  freeze(ms: number | null): void {
    frozen = ms;
  },
};
