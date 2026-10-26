export const pixelRatio = (): number => Math.max(1, Math.round(window.devicePixelRatio || 1));

export function watchPixelRatio(changed: (ratio: number) => void): void {
  const watch = () => {
    // Observe the raw ratio: fractional transitions must re-arm even at the same integer scale.
    window.matchMedia(`(resolution: ${window.devicePixelRatio || 1}dppx)`).addEventListener("change", () => {
      watch();
      changed(pixelRatio());
    }, { once: true });
  };
  watch();
}
