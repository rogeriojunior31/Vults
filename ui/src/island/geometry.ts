// Preferred logical widths never resize the mapped overlay or override available screen space.
export const OPEN_WIDTH = 440;
export const CHAT_WIDTH = 492;
export const OPEN_ZOOM = 1.14;
export const DICTATION_HEIGHT = 24;
const MAX_HEIGHT = 556;
export const ISLAND_WIDTHS = [0, 560, 680, 760] as const;
export type IslandWidth = typeof ISLAND_WIDTHS[number];
const WIDTH_KEY = "island-width";

export function islandWidth(value: unknown): IslandWidth {
  return ISLAND_WIDTHS.includes(value as IslandWidth) ? value as IslandWidth : 0;
}

export function restoreWidth(): IslandWidth {
  try { return islandWidth(JSON.parse(localStorage.getItem(WIDTH_KEY) ?? "0")); }
  catch { return 0; }
}

export function saveWidth(value: IslandWidth): boolean {
  try { localStorage.setItem(WIDTH_KEY, String(value)); return true; }
  catch { return false; }
}

/** Webviews share presentation storage; refocus also catches missed storage notifications. */
export function watchWidth(changed: (value: IslandWidth) => void): void {
  let current = restoreWidth();
  const refresh = () => {
    const next = restoreWidth();
    if (next !== current) { current = next; changed(next); }
  };
  window.addEventListener("storage", event => {
    if (event.key === WIDTH_KEY || event.key === null) {
      current = restoreWidth();
      changed(current);
    }
  });
  window.addEventListener("focus", refresh);
}

export function miniWidth(available: number, preferred: IslandWidth = 0): number {
  return Math.max(0, Math.min(preferred || 340, Math.floor(available)));
}

/** Keep bird hit targets apart and reserve the trailing action only when sessions overflow. */
export function miniSessions(width: number, total: number): { visible: number; hidden: number } {
  const places = (reserved: number) => Math.max(0, Math.min(8, Math.floor((width - 52 - 38 - reserved) / 32) + 1));
  const visible = Math.min(total, places(total > places(0) ? 40 : 0));
  return { visible, hidden: total - visible };
}

export function miniHeight(available: number): number {
  return Math.max(0, Math.min(38, Math.floor(available)));
}

/** CSS sizes before zoom. Round down so the measured rectangle never exceeds the surface. */
export function quickBounds(width: number, height: number, inset = 0, preferred: IslandWidth = 0): { width: number; chatWidth: number; height: number } {
  const availableWidth = Math.max(0, Math.floor(width / OPEN_ZOOM));
  return {
    width: Math.min(preferred ? Math.floor(preferred / OPEN_ZOOM) : OPEN_WIDTH, availableWidth),
    chatWidth: Math.min(preferred ? Math.floor(preferred / OPEN_ZOOM) : CHAT_WIDTH, availableWidth),
    height: Math.max(0, Math.min(preferred ? Math.floor((600 - inset) / OPEN_ZOOM) : MAX_HEIGHT, Math.floor((height - inset) / OPEN_ZOOM))),
  };
}

/** GTK input regions are integral logical pixels, including the UI's zoom exactly once. */
export function quickPixels(width: number, height: number, inset = 0): { w: number; h: number } {
  return { w: Math.ceil(width * OPEN_ZOOM), h: Math.ceil(height * OPEN_ZOOM) + inset };
}

/** Captions use spare space outside the island, without changing the input rectangle. */
export function dictationCaptionTop(anchor: Pick<DOMRect, "top" | "bottom">, height: number, viewport: number, bottom: boolean): number | null {
  const top = bottom ? anchor.top - height - 6 : anchor.bottom + 6;
  return height > 0 && top >= 6 && top + height <= viewport - 6 ? top : null;
}
