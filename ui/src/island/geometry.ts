// Quick stays inside the mapped native overlay; the chat can use more of its width.
export const OPEN_WIDTH = 440;
export const CHAT_WIDTH = 492;
export const OPEN_ZOOM = 1.14;
const MAX_HEIGHT = 556;

/** CSS sizes before zoom. Round down so the measured rectangle never exceeds the surface. */
export function quickBounds(width: number, height: number): { width: number; chatWidth: number; height: number } {
  const availableWidth = Math.max(0, Math.floor(width / OPEN_ZOOM));
  return {
    width: Math.min(OPEN_WIDTH, availableWidth),
    chatWidth: Math.min(CHAT_WIDTH, availableWidth),
    height: Math.min(MAX_HEIGHT, Math.max(0, Math.floor(height / OPEN_ZOOM))),
  };
}

/** GTK input regions are integral logical pixels, including the UI's zoom exactly once. */
export function quickPixels(width: number, height: number): { w: number; h: number } {
  return { w: Math.ceil(width * OPEN_ZOOM), h: Math.ceil(height * OPEN_ZOOM) };
}
