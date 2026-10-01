// The wire at the top of the island: Zeca (the focused session) at 2x, the vults (every other
// session) at 1x. Birds arrive by flying in and leave by flying off; idle ones doze. It redraws
// only when a frame changes and stops when there is nothing to show, so a quiet island costs no
// CPU. Timers, not requestAnimationFrame: WebKit pauses rAF while it believes the layer-shell
// surface is hidden.
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
/** An idle bird dozes off after this long. */
const NAP_MS = 90_000;

interface Flock {
  bird: Bird;
  agent: SessionView["agent"];
  clip: string;
  /** When it started idling, for the nap. */
  idleSince: number | null;
}

export class Scene {
  readonly canvas = document.createElement("canvas");
  private readonly ctx: CanvasRenderingContext2D;
  private readonly dpr = Math.max(1, Math.round(window.devicePixelRatio || 1));
  private zeca: Flock | null = null;
  private readonly vults = new Map<string, Flock & { slot: number; leaving: boolean }>();
  private timer: number | undefined;

  constructor() {
    this.canvas.className = "scene";
    this.canvas.width = SCENE_W * this.dpr;
    this.canvas.height = SCENE_H * this.dpr;
    this.canvas.style.width = `${SCENE_W}px`;
    this.canvas.style.height = `${SCENE_H}px`;
    this.ctx = this.canvas.getContext("2d")!;
  }

  /**
   * `sessions` most recent first; the focused one is Zeca, the rest are vults. `talking` puts
   * Zeca on the wire for the chat instead (a clip and the provider's band).
   */
  update(sessions: SessionView[], focus: SessionView | null, talking: { clip: string; agent: SessionView["agent"] } | null = null): void {
    const now = performance.now();

    if (!focus && !talking) {
      this.zeca = null;
      this.vults.clear();
      this.schedule(null);
      return;
    }
    if (!this.zeca) {
      const bird = new Bird(ZECA, {
        x: 8,
        wireY: WIRE / ZECA_SCALE,
        height: PERCH_HEIGHT,
        skyRight: SCENE_W / ZECA_SCALE,
        skyTop: 0,
      });
      bird.arrive(now);
      this.zeca = { bird, agent: (focus ?? talking)!.agent, clip: "", idleSince: null };
    }
    this.zeca.agent = focus ? focus.agent : talking!.agent;
    this.want(this.zeca, focus ? clipFor(focus) : talking!.clip, now);

    const others = sessions.filter((s) => s !== focus).slice(0, MAX_VULTS);
    const present = new Set(others.map(key));
    for (const [k, v] of this.vults) {
      if (!present.has(k) && !v.leaving) {
        v.leaving = true;
        v.bird.leave(now);
      }
    }
    for (const s of others) {
      const k = key(s);
      let v = this.vults.get(k);
      if (!v || v.leaving) {
        const slot = this.freeSlot();
        const bird = new Bird(ZECA, {
          x: 76 + slot * 34,
          wireY: WIRE / VULT_SCALE,
          height: PERCH_HEIGHT,
          skyRight: SCENE_W / VULT_SCALE,
          skyTop: 4,
        });
        bird.arrive(now);
        v = { bird, agent: s.agent, clip: "", idleSince: null, slot, leaving: false };
        this.vults.set(k, v);
      }
      v.agent = s.agent;
      this.want(v, clipFor(s), now);
    }
    this.schedule(0);
  }

  private want(f: Flock, clip: string, now: number): void {
    if (clip === f.clip) return;
    f.clip = clip;
    f.idleSince = clip === "idle" ? now : null;
    f.bird.want(clip, now);
  }

  private freeSlot(): number {
    const used = new Set([...this.vults.values()].filter((v) => !v.leaving).map((v) => v.slot));
    let slot = 0;
    while (used.has(slot)) slot++;
    return slot;
  }

  private schedule(ms: number | null): void {
    window.clearTimeout(this.timer);
    this.timer = ms === null ? undefined : window.setTimeout(() => this.draw(), ms);
  }

  private draw(): void {
    const now = performance.now();
    const ctx = this.ctx;
    const css = getComputedStyle(this.canvas);
    const accent = (agent: SessionView["agent"]) => ({
      A: css.getPropertyValue(`--agent-${agent}`).trim() || "#d97757",
    });
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    ctx.fillStyle = css.getPropertyValue("--wire").trim() || "#3a3a40";
    ctx.fillRect(0, WIRE * this.dpr, this.canvas.width, this.dpr);

    let next = 1000;
    const nap = (f: Flock) => {
      if (f.idleSince !== null && now - f.idleSince > NAP_MS) f.bird.want("sleep", now);
      else if (f.idleSince !== null) next = Math.min(next, f.idleSince + NAP_MS - now + 1);
    };

    for (const [k, v] of this.vults) {
      if (v.bird.gone(now)) {
        this.vults.delete(k);
        continue;
      }
      nap(v);
      const s = v.bird.shot(now);
      drawFrame(ctx, ZECA, s.frame, s.x, s.y, VULT_SCALE * this.dpr, s.flip, accent(v.agent));
      next = Math.min(next, v.bird.nextChange(now));
    }
    if (this.zeca) {
      nap(this.zeca);
      const z = this.zeca.bird.shot(now);
      drawFrame(ctx, ZECA, z.frame, z.x, z.y, ZECA_SCALE * this.dpr, z.flip, accent(this.zeca.agent));
      next = Math.min(next, this.zeca.bird.nextChange(now));
    }
    this.schedule(next);
  }
}

const key = (s: SessionView) => `${s.agent}:${s.id}`;
