// Pixel sprites made of parts: a frame stacks parts at integer offsets. The engine knows
// nothing about vultures; the data (zeca/zeca.json) does.

export type Grid = string[];
export type Layer = [part: string, x: number, y: number];

export interface Frame {
  ms: number;
  /** Whole-bird offset: hops, leans, shakes. */
  dx: number;
  dy: number;
  layers: Layer[];
}

export interface Clip {
  loop: boolean;
  frames: Frame[];
}

export interface SpriteSet {
  palette: Record<string, string>;
  parts: Record<string, Grid>;
  clips: Record<string, Clip>;
}

export function clipLength(clip: Clip): number {
  return clip.frames.reduce((sum, f) => sum + f.ms, 0);
}

/** The frame showing `t` ms after the clip started. A one-shot clip holds its last frame. */
export function frameAt(clip: Clip, t: number): Frame {
  const total = clipLength(clip);
  let left = clip.loop ? t % total : Math.min(t, total - 1);
  for (const f of clip.frames) {
    if (left < f.ms) return f;
    left -= f.ms;
  }
  return clip.frames[clip.frames.length - 1];
}

/** Width of the widest layer, for mirroring a frame around its own box. */
function frameWidth(set: SpriteSet, frame: Frame): number {
  return Math.max(...frame.layers.map(([p, x]) => x + Math.max(...set.parts[p].map((r) => r.length))));
}

/**
 * Draws a frame with its top-left at grid cell (x, y), each cell `scale` device pixels.
 * `flip` mirrors it (facing left).
 */
export function drawFrame(
  ctx: CanvasRenderingContext2D,
  set: SpriteSet,
  frame: Frame,
  x: number,
  y: number,
  scale: number,
  flip = false,
): void {
  const width = flip ? frameWidth(set, frame) : 0;
  for (const [part, lx, ly] of frame.layers) {
    const grid = set.parts[part];
    if (!grid) continue;
    for (let row = 0; row < grid.length; row++) {
      const line = grid[row];
      for (let col = 0; col < line.length; col++) {
        const color = set.palette[line[col]];
        if (!color || line[col] === ".") continue;
        const gx = flip ? width - 1 - (lx + col) : lx + col;
        ctx.fillStyle = color;
        ctx.fillRect((x + frame.dx + gx) * scale, (y + frame.dy + ly + row) * scale, scale, scale);
      }
    }
  }
}
