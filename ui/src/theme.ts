import type { Theme } from "./bridge";

export const DEFAULT_THEME: Theme = "dark";

let current: Theme = DEFAULT_THEME;
let changed = false;

/** A late startup read must not undo a theme event already received. */
export function applyTheme(theme: Theme | undefined, initial = false): Theme {
  if (!theme || initial && changed) return current;
  current = theme;
  changed ||= !initial;
  const different = document.documentElement?.getAttribute("data-theme") !== theme;
  document.documentElement?.setAttribute("data-theme", theme);
  if (different) document.dispatchEvent(new Event("themechange"));
  return current;
}
