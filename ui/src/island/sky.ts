// One persistent bird per idle session, shared by the compact and open island.
import { Bird, type Perch } from "../character/director";
import { FrameCache } from "../character/sprites";
import { ZECA, PERCH_HEIGHT } from "../character/zeca";
import { Clock } from "../clock";
import type { SessionView } from "../bridge";
import { clipFor } from "./behavior";

export type SkyPerch = { x: number; y: number; scale: number };
const key = (s: SessionView) => `${s.agent}:${s.id}`;
const IDLE_MS = 2000;
const WIDTH = 720;
const HEIGHT = 560;

export class Sky {
  readonly canvas = document.createElement("canvas");
  private readonly ctx: CanvasRenderingContext2D;
  private readonly cache = new FrameCache(ZECA);
  private readonly birds = new Map<string, {
    bird: Bird; session: SessionView; idleAt: number | null; airborne: boolean;
    landingAt: number | null; scale: number; leaving: boolean; hash: number;
    takeoffAt: number; launchScale: number;
  }>();
  private readonly dpr = Math.max(1, Math.round(devicePixelRatio || 1));
  private timer: number | undefined;
  private active = true;
  private readonly motion = matchMedia("(prefers-reduced-motion: reduce)");
  private anchors = new Map<string, SkyPerch>();
  private skyY = 100;
  private targetY = 100;
  private colors: Record<string, string> | null = null;

  constructor(private readonly changed: () => void) {
    this.canvas.className = "scene flock-sky";
    this.canvas.setAttribute("aria-hidden", "true");
    this.canvas.width = WIDTH * this.dpr;
    this.canvas.height = HEIGHT * this.dpr;
    this.ctx = this.canvas.getContext("2d")!;
    this.motion.addEventListener("change", () => this.draw());
  }

  owns(id: string): boolean { return this.birds.get(id)?.airborne ?? false; }

  update(sessions: SessionView[], active: boolean): void {
    const now = Clock.now();
    this.active = active;
    const present = new Set(sessions.map(key));
    for (const [id, f] of this.birds) {
      if (present.has(id) || f.leaving) continue;
      if (f.airborne) { f.bird.leave(now); f.leaving = true; }
      else this.birds.delete(id);
    }
    for (const session of sessions) {
      const id = key(session);
      let f = this.birds.get(id);
      if (!f || f.leaving) {
        const anchor = this.anchors.get(id) ?? { x: WIDTH / 2, y: 22, scale: 1 };
        let hash = 2166136261;
        for (const c of id) hash = Math.imul(hash ^ c.charCodeAt(0), 16777619) >>> 0;
        f = { bird: new Bird(ZECA, this.perch(anchor)), session, idleAt: null,
          airborne: false, landingAt: null, scale: anchor.scale, leaving: false, hash,
          takeoffAt: now, launchScale: anchor.scale };
        this.birds.set(id, f);
      }
      const wasIdle = f.idleAt !== null;
      f.session = session;
      if (session.status === "idle") {
        if (!wasIdle) { f.idleAt = now; if (!f.bird.isSoaring()) f.bird.want("idle", now); }
      } else {
        f.idleAt = null;
        if (f.bird.isSoaring()) {
          // Keep web work on its perch after landing; the scene owns its next sortie.
          f.bird.want(clipFor(session) === "fly" ? "idle" : clipFor(session), now);
          f.landingAt = now;
        }
      }
    }
    this.draw();
  }

  place(anchors: Map<string, SkyPerch>, height: number): void {
    this.anchors = anchors;
    this.targetY = Math.min(HEIGHT - 65, height + 70);
    const now = Clock.now();
    for (const [id, f] of this.birds) {
      const at = anchors.get(id);
      if (at) {
        f.scale = at.scale;
        f.bird.movePerch(this.perch(at), now);
      }
    }
  }

  private perch(at: SkyPerch): Perch {
    return { x: at.x - 10, wireY: at.y + PERCH_HEIGHT - 6,
      height: PERCH_HEIGHT, skyRight: WIDTH, skyTop: 8 };
  }

  private draw(): void {
    clearTimeout(this.timer);
    const ctx = this.ctx, now = Clock.now();
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    const calm = this.motion.matches || document.body.classList.contains("still");
    let changed = false, next = 1000;
    this.skyY += (this.targetY - this.skyY) * 0.12;
    if (!this.colors) {
      const css = getComputedStyle(this.canvas);
      this.colors = Object.fromEntries(["claude", "codex", "other"].map(agent =>
        [agent, css.getPropertyValue(`--agent-${agent}`).trim()]));
    }
    for (const [id, f] of this.birds) {
      if (calm) {
        if (f.airborne) { f.airborne = false; changed = true; }
        f.bird = new Bird(ZECA, this.perch(this.anchors.get(id) ?? { x: 360, y: 22, scale: 1 }));
        continue;
      }
      if (!this.active) continue;
      if (f.leaving && f.bird.gone(now)) { this.birds.delete(id); changed = true; continue; }
      if (!f.leaving && f.idleAt !== null && ((!f.airborne && now - f.idleAt >= IDLE_MS) || f.bird.isSoaring())) {
        const hash = f.hash;
        // Separate phases and nested orbits keep an idle flock from moving in lockstep.
        f.bird.soar({ cx: 360, cy: this.skyY + hash % 3 * 8, rx: 250 - hash % 5 * 16,
          ry: 38, lapMs: 9000 + hash % 4 * 700, phase: hash % 360 * Math.PI / 180 }, now);
        if (!f.airborne) {
          f.airborne = true; f.landingAt = null; f.takeoffAt = now; f.launchScale = f.scale; changed = true;
        }
      }
      if (!f.airborne) continue;
      const shot = f.bird.shot(now);
      if (!f.bird.isFlying()) { f.airborne = false; changed = true; continue; }
      const landing = f.landingAt === null ? 0 : Math.min(1, (now - f.landingAt) / 600);
      const launch = Math.min(1, (now - f.takeoffAt) / 1800);
      const scale = f.landingAt === null ? f.launchScale + (1 - f.launchScale) * launch : 1 + (f.scale - 1) * landing;
      const flying = shot.frame.layers.length === 1;
      const cx = flying ? 18.5 : 10, cy = flying ? 8 : 6;
      ctx.save();
      ctx.translate((shot.x + cx) * this.dpr, (shot.y + cy) * this.dpr);
      ctx.scale(scale, scale);
      const accent = this.colors[f.session.agent];
      this.cache.drawShot(ctx, { ...shot, x: -cx, y: -cy }, this.dpr, accent ? { A: accent } : undefined);
      ctx.restore();
      next = Math.min(next, f.bird.nextChange(now));
    }
    if (changed) this.changed();
    if (this.active && !calm && this.birds.size) this.timer = window.setTimeout(() => this.draw(), next);
  }
}
