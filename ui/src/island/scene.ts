// The wire at the top of the island: Zeca (the focused session) at 2x, the vults (every other
// session) at 1x. It redraws only when a frame changes and stops when there is nothing to show,
// so a quiet island costs no CPU. Timers, not requestAnimationFrame: WebKit pauses rAF while it
// believes the layer-shell surface is hidden.
import type { SessionView } from "../bridge";
import { Bird } from "../character/director";
import { drawFrame } from "../character/sprites";
import { PERCH_HEIGHT, ZECA } from "../character/zeca";
import { clipFor } from "./behavior";

/** CSS pixels. */
export const SCENE_W = 300;
export const SCENE_H = 64;
const ZECA_SCALE = 2;
const VULT_SCALE = 1;
/** The wire's row, in CSS pixels. */
const WIRE = 58;
const MAX_VULTS = 6;

export class Scene {
  readonly canvas = document.createElement("canvas");
  private readonly ctx: CanvasRenderingContext2D;
  private readonly dpr = Math.max(1, Math.round(window.devicePixelRatio || 1));
  private readonly zeca = new Bird(ZECA, {
    x: 8,
    wireY: WIRE / ZECA_SCALE,
    height: PERCH_HEIGHT,
    skyRight: SCENE_W / ZECA_SCALE,
    skyTop: 0,
  });
  private vults = new Map<string, Bird>();
  private order: string[] = [];
  private timer: number | undefined;
  private active = false;

  constructor() {
    this.canvas.className = "scene";
    this.canvas.width = SCENE_W * this.dpr;
    this.canvas.height = SCENE_H * this.dpr;
    this.canvas.style.width = `${SCENE_W}px`;
    this.canvas.style.height = `${SCENE_H}px`;
    this.ctx = this.canvas.getContext("2d")!;
  }

  /** `sessions` most recent first; the focused one is Zeca, the rest are vults. */
  update(sessions: SessionView[], focus: SessionView | null): void {
    const now = performance.now();
    this.active = focus !== null;
    if (focus) this.zeca.want(clipFor(focus), now);

    const others = sessions.filter((s) => s !== focus).slice(0, MAX_VULTS);
    const keep = new Set(others.map(key));
    for (const k of [...this.vults.keys()]) if (!keep.has(k)) this.vults.delete(k);
    this.order = others.map(key);
    others.forEach((s, i) => {
      const k = key(s);
      let bird = this.vults.get(k);
      if (!bird) {
        bird = new Bird(ZECA, {
          x: 76 + i * 34,
          wireY: WIRE / VULT_SCALE,
          height: PERCH_HEIGHT,
          skyRight: SCENE_W / VULT_SCALE,
          skyTop: 4,
        });
        this.vults.set(k, bird);
      }
      bird.want(clipFor(s), now);
    });
    this.schedule(0);
  }

  private schedule(ms: number): void {
    window.clearTimeout(this.timer);
    if (!this.active) {
      this.timer = undefined;
      return;
    }
    this.timer = window.setTimeout(() => this.draw(), ms);
  }

  private draw(): void {
    const now = performance.now();
    const ctx = this.ctx;
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    ctx.fillStyle = getComputedStyle(this.canvas).getPropertyValue("--wire").trim() || "#3a3a40";
    ctx.fillRect(0, WIRE * this.dpr, this.canvas.width, this.dpr);

    let next = 1000;
    for (const k of this.order) {
      const bird = this.vults.get(k);
      if (!bird) continue;
      const s = bird.shot(now);
      drawFrame(ctx, ZECA, s.frame, s.x, s.y, VULT_SCALE * this.dpr, s.flip);
      next = Math.min(next, bird.nextChange(now));
    }
    const z = this.zeca.shot(now);
    drawFrame(ctx, ZECA, z.frame, z.x, z.y, ZECA_SCALE * this.dpr, z.flip);
    next = Math.min(next, this.zeca.nextChange(now));
    this.schedule(next);
  }
}

const key = (s: SessionView) => `${s.agent}:${s.id}`;
