// Quick stays inside the mapped native overlay; the chat can use more of its width.
export const OPEN_WIDTH = 440;
export const CHAT_WIDTH = 492;
export const OPEN_ZOOM = 1.14;
export const DICTATION_HEIGHT = 24;
const MAX_HEIGHT = 556;

/** CSS sizes before zoom. Round down so the measured rectangle never exceeds the surface. */
export function quickBounds(width: number, height: number, inset = 0): { width: number; chatWidth: number; height: number } {
  const availableWidth = Math.max(0, Math.floor(width / OPEN_ZOOM));
  return {
    width: Math.min(OPEN_WIDTH, availableWidth),
    chatWidth: Math.min(CHAT_WIDTH, availableWidth),
    height: Math.min(MAX_HEIGHT, Math.max(0, Math.floor((height - inset) / OPEN_ZOOM))),
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
