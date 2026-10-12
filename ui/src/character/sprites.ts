// Pixel sprites made of parts: a frame stacks parts at integer offsets. The engine knows
// nothing about vultures; the data (zeca/zeca.json) does.
import type { Shot } from "./director";

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
  /** Marks drawn over the head (a thought bubble, a bang): clips of their own. */
  emotes?: Record<string, Clip>;
  /** Rows from the body's top-left down to the wire, when not Zeca's. */
  perch?: number;
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

  drawShot(ctx: CanvasRenderingContext2D, shot: Shot, scale: number, colors?: Record<string, string>): void {
    ctx.save();
    ctx.imageSmoothingEnabled = false;
    ctx.globalAlpha = shot.alpha ?? 1;
    // A banking bird is drawn, not rotated: whole cells, the same size as when it flies level.
    const step = Math.max(-2, Math.min(2, Math.round((shot.bank ?? 0) / 0.05)));
    const frame = step ? this.banked(shot.frame, shot.flip ? -step : step) : shot.frame;
    this.draw(ctx, frame, shot.x, shot.y, scale, shot.flip, colors);
    ctx.restore();
  }

  private readonly banks = new WeakMap<Frame, Map<number, Frame>>();
  private static bankId = 0;
  /** The frame banked `step` steps (clockwise when positive): composed into one part and rolled. */
  private banked(frame: Frame, step: number): Frame {
    let byStep = this.banks.get(frame);
    if (!byStep) this.banks.set(frame, (byStep = new Map()));
    let out = byStep.get(step);
    if (!out) {
      const { grid, x, y } = compose(this.set, frame);
      const r = roll(grid, step);
      const name = `~bank${FrameCache.bankId++}`;
      this.set.parts[name] = r.grid;
      out = { ...frame, layers: [[name, x + r.dx, y + r.dy]] };
      byStep.set(step, out);
    }
    return out;
  }

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

/** A frame's layers flattened into one grid, the later ones over the earlier, with its top-left. */
function compose(set: SpriteSet, frame: Frame): { grid: Grid; x: number; y: number } {
  const cells = new Map<string, string>();
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const [part, lx, ly] of frame.layers)
    (set.parts[part] ?? []).forEach((r, j) =>
      [...r].forEach((c, i) => {
        if (c === ".") return;
        cells.set(`${lx + i},${ly + j}`, c);
        minX = Math.min(minX, lx + i);
        minY = Math.min(minY, ly + j);
        maxX = Math.max(maxX, lx + i);
        maxY = Math.max(maxY, ly + j);
      }),
    );
  if (!cells.size) return { grid: [], x: 0, y: 0 };
  const grid = Array.from({ length: maxY - minY + 1 }, (_, j) =>
    Array.from({ length: maxX - minX + 1 }, (_, i) => cells.get(`${minX + i},${minY + j}`) ?? ".").join(""),
  );
  return { grid, x: minX, y: minY };
}

/**
 * A flight grid banked `step` steps (-2 to 2; clockwise when positive): the wing on the high side
 * lifts and the low one drops, each column by its distance from the body, and the low wing, turned
 * away, loses a few columns. `dx`, `dy` keep the body where it was.
 */
export function roll(g: Grid, step: number): { grid: Grid; dx: number; dy: number } {
  const w = Math.max(0, ...g.map((r) => r.length)),
    cx = Math.floor(w / 2),
    core = 6,
    slope = 0.09 * Math.abs(step),
    lift = Math.round((cx - core) * slope);
  // Clockwise: the left wing goes up, the right one down and away.
  const lowLeft = step < 0;
  const drop = Math.round(Math.abs(step) * (cx - core) * 0.07);
  const lowCols = new Set<number>();
  for (let k = 1; k <= drop; k++) {
    const d = core + Math.round(((cx - core) * k) / (drop + 1));
    lowCols.add(lowLeft ? cx - d : cx + d);
  }
  const h = g.length + 2 * lift;
  const out = Array.from({ length: h }, () => [] as string[]);
  for (let x = 0; x < w; x++) {
    if (lowCols.has(x)) continue;
    const d = x - cx,
      far = Math.max(0, Math.abs(d) - core),
      up = (d < 0) !== lowLeft;
    const k = Math.round(far * slope) * (up ? -1 : 1);
    const col = Array<string>(h).fill(".");
    for (let y = 0; y < g.length; y++) {
      const c = g[y][x] ?? ".";
      if (c !== ".") col[y + lift + k] = c;
    }
    col.forEach((c, y) => out[y].push(c));
  }
  // The box keeps its width, so a mirrored frame keeps the body where it was.
  const pad = lowLeft ? "" : ".".repeat(drop);
  return { grid: out.map((r) => r.join("") + pad), dx: lowLeft ? drop : 0, dy: -lift };
}
