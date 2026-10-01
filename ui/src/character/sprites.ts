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
export function frameWidth(set: SpriteSet, frame: Frame): number {
  return Math.max(...frame.layers.map(([p, x]) => x + Math.max(...set.parts[p].map((r) => r.length))));
}

/**
 * Draws a frame with its top-left at grid cell (x, y), each cell `scale` device pixels.
 * `flip` mirrors it (facing left); `colors` overrides palette entries (the agent's accent).
 */
export function drawFrame(
  ctx: CanvasRenderingContext2D,
  set: SpriteSet,
  frame: Frame,
  x: number,
  y: number,
  scale: number,
  flip = false,
  colors?: Record<string, string>,
): void {
  const palette = colors ? { ...set.palette, ...colors } : set.palette;
  const width = flip ? frameWidth(set, frame) : 0;
  for (const [part, lx, ly] of frame.layers) {
    const grid = set.parts[part];
    if (!grid) continue;
    for (let row = 0; row < grid.length; row++) {
      const line = grid[row];
      for (let col = 0; col < line.length; col++) {
        const color = palette[line[col]];
        if (!color || line[col] === ".") continue;
        const gx = flip ? width - 1 - (lx + col) : lx + col;
        ctx.fillStyle = color;
        ctx.fillRect((x + frame.dx + gx) * scale, (y + frame.dy + ly + row) * scale, scale, scale);
      }
    }
  }
}

/**
 * Draws a frame from a cache of ready-made bitmaps: one per frame, scale, flip and accent, so a
 * redraw is a single drawImage instead of hundreds of fillRect calls. Frames are reused objects
 * from the sprite data, which makes them good cache keys.
 */
export class FrameCache {
  private readonly cache = new WeakMap<Frame, Map<string, { canvas: HTMLCanvasElement; ox: number; oy: number }>>();

  constructor(private readonly set: SpriteSet) {}

  draw(
    ctx: CanvasRenderingContext2D,
    frame: Frame,
    x: number,
    y: number,
    scale: number,
    flip = false,
    colors?: Record<string, string>,
  ): void {
    const key = `${scale}|${flip ? 1 : 0}|${colors ? Object.values(colors).join(",") : ""}`;
    let byKey = this.cache.get(frame);
    if (!byKey) {
      byKey = new Map();
      this.cache.set(frame, byKey);
    }
    let entry = byKey.get(key);
    if (!entry) {
      entry = this.render(frame, scale, flip, colors);
      byKey.set(key, entry);
    }
    ctx.drawImage(entry.canvas, (x + frame.dx + entry.ox) * scale, (y + frame.dy + entry.oy) * scale);
  }

  /** The frame into its own canvas, trimmed to the cells it covers (offsets may be negative). */
  private render(frame: Frame, scale: number, flip: boolean, colors?: Record<string, string>) {
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const [part, lx, ly] of frame.layers) {
      const grid = this.set.parts[part] ?? [];
      minX = Math.min(minX, lx);
      minY = Math.min(minY, ly);
      maxX = Math.max(maxX, lx + Math.max(0, ...grid.map((r) => r.length)));
      maxY = Math.max(maxY, ly + grid.length);
    }
    const w = flip ? frameWidth(this.set, frame) : maxX;
    const ox = flip ? Math.min(0, w - maxX) : minX;
    const width = (flip ? Math.max(w, w - minX) : maxX) - ox;
    const canvas = document.createElement("canvas");
    canvas.width = Math.max(1, width * scale);
    canvas.height = Math.max(1, (maxY - minY) * scale);
    const c = canvas.getContext("2d")!;
    drawFrame(c, this.set, { ...frame, dx: 0, dy: 0 }, -ox, -minY, scale, flip, colors);
    return { canvas, ox, oy: minY };
  }
}
