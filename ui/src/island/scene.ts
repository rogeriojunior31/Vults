// Birds on a wire, in one canvas: Zeca (the focused session) and the vults (the other sessions).
// The island has three: the compact pill (everyone on one wire), the focus card (Zeca alone, on
// his piece of wire) and the flock list (one vult per row, each on a short perch). Birds arrive by
// flying in and leave by flying off where there is room; idle ones doze. A scene redraws only when
// a frame changes, and not at all while it is off screen, so a quiet island costs no CPU. Timers,
// not requestAnimationFrame: WebKit pauses rAF while it believes the layer-shell surface is hidden.
import { Clock } from "../clock";
import type { SessionView } from "../bridge";
import { Bird, type Thermal } from "../character/director";
import { FrameCache } from "../character/sprites";
import { PERCH_HEIGHT, ZECA } from "../character/zeca";
import { clipFor } from "./behavior";

/** Where everything sits, in CSS pixels. */
export interface SceneLayout {
  width: number;
  height: number;
  /** A wire across the whole canvas at this row; null puts a short perch under each bird. */
  wire: number | null;
  zeca: { x: number; wire: number; scale: number } | null;
  vults: {
    scale: number;
    max: number;
    at: (slot: number) => { x: number; wire: number };
  };
  /** Highest row a flight may reach. */
  skyTop: number;
  /** Birds fly in and off (it needs room); otherwise they just appear and go. */
  flights: boolean;
  /** A vult sits at its place in the list, and moves up when one above it leaves. */
  reflow: boolean;
  /** Where idle birds circle, waiting for something to do; null keeps them on the wire. */
  sky: { cx: number; cy: number; rx: number; ry: number } | null;
}

/** Width of a perched bird at 1x, in CSS pixels. */
export const BIRD_W = 24;

/** The compact island: Zeca on the left, up to four vults roosting close on the right. */
export const COMPACT_W = 360;
export const COMPACT_H = 38;
export const COMPACT_SCENE: SceneLayout = {
  width: COMPACT_W,
  height: COMPACT_H,
  wire: 33,
  zeca: { x: 12, wire: 33, scale: 1 },
  vults: {
    scale: 1,
    max: 4,
    at: (slot) => ({ x: COMPACT_W - 14 - BIRD_W - slot * 25, wire: 33 }),
  },
  skyTop: 0,
  flights: true,
  reflow: false,
  // One thermal over the whole pill, behind the text.
  sky: { cx: COMPACT_W / 2, cy: 13, rx: 128, ry: 4 },
};

/** The focus card: Zeca large, on his own piece of wire. */
export const FOCUS_SCENE: SceneLayout = {
  width: 96,
  height: 76,
  wire: 64,
  zeca: { x: 24, wire: 64, scale: 2 },
  vults: { scale: 1, max: 0, at: () => ({ x: 0, wire: 0 }) },
  skyTop: 0,
  flights: false,
  reflow: false,
  sky: null,
};

/** The flock list: one vult per row. */
export const LIST_ROW = 32;
export const LIST_SCENE: SceneLayout = {
  width: 30,
  height: LIST_ROW,
  wire: null,
  zeca: null,
  vults: {
    scale: 1,
    max: 32,
    at: (slot) => ({ x: 3, wire: slot * LIST_ROW + 26 }),
  },
  skyTop: 0,
  flights: false,
  reflow: true,
  sky: null,
};

/** No flights when the user asked for less motion (or the lab takes a still). */
const calm = () =>
  window.matchMedia("(prefers-reduced-motion: reduce)").matches ||
  document.body.classList.contains("still");
/** Where there is no sky (or no motion), an idle bird dozes off after this long. */
const NAP_MS = 90_000;
/** Under a sky, an idle bird takes off after this long and circles, waiting for work… */
const SOAR_AFTER_MS = 20_000;
/** …and after this long with nothing, comes back to roost on the wire and dozes. */
const SOAR_FOR_MS = 5 * 60_000;

interface Flock {
  bird: Bird;
  /** Its session's key ("zeca" for Zeca): its place in the thermal comes from it. */
  key: string;
  agent: SessionView["agent"];
  clip: string;
  /** When it started idling, for the nap or the soaring. */
  idleSince: number | null;
  /** When it took off to soar; null on the wire. */
  soaredAt: number | null;
  /** Back from soaring, dozing: it stays down until it has work. */
  roosting: boolean;
}

/** A small stable number from a key, so each bird keeps its own place, size and pace of lap. */
function hash(key: string): number {
  let h = 2166136261;
  for (let i = 0; i < key.length; i++) h = Math.imul(h ^ key.charCodeAt(i), 16777619);
  return h >>> 0;
}

type Vult = Flock & { slot: number; leaving: boolean };

export class Scene {
  readonly canvas = document.createElement("canvas");
  private readonly ctx: CanvasRenderingContext2D;
  private readonly dpr = Math.max(1, Math.round(window.devicePixelRatio || 1));
  private readonly layout: SceneLayout;
  private zeca: Flock | null = null;
  private readonly vults = new Map<string, Vult>();
  private timer: number | undefined;
  private readonly frames = new FrameCache(ZECA);
  /** Theme colors, read once: getComputedStyle on every frame is not free. */
  private colors: { wire: string; claude: string; codex: string } | null = null;
  /** Off screen: birds still follow their sessions, but nothing is drawn. */
  private active = true;

  constructor(layout: SceneLayout) {
    this.layout = { ...layout };
    this.canvas.className = "scene";
    this.ctx = this.canvas.getContext("2d")!;
    this.size(layout.height);
  }

  /** The list grows with the flock. */
  setHeight(height: number): void {
    if (height === this.layout.height) return;
    this.layout.height = height;
    this.size(height);
    this.schedule(0);
  }

  private size(height: number): void {
    this.canvas.width = this.layout.width * this.dpr;
    this.canvas.height = height * this.dpr;
    this.canvas.style.width = `${this.layout.width}px`;
    this.canvas.style.height = `${height}px`;
  }

  /** Starts or stops drawing; clips are timed by the clock, so they pick up where they are. */
  setActive(on: boolean): void {
    if (on === this.active) return;
    this.active = on;
    this.schedule(on ? 0 : null);
  }

  /** Where each perched bird is, in CSS pixels. Zeca's key is "zeca". */
  slots(): { key: string; x: number; width: number }[] {
    const { zeca, vults } = this.layout;
    const out = [];
    if (this.zeca && zeca)
      out.push({ key: "zeca", x: zeca.x, width: BIRD_W * zeca.scale });
    for (const [k, v] of this.vults) {
      if (!v.leaving)
        out.push({
          key: k,
          x: vults.at(v.slot).x,
          width: BIRD_W * vults.scale,
        });
    }
    return out;
  }

  /**
   * `sessions` in their order on the wire; `focus` is Zeca (when this scene has him), the rest are
   * vults. `talking` puts Zeca on the wire with no session (the chat, an empty wire).
   */
  update(
    sessions: SessionView[],
    focus: SessionView | null,
    talking: { clip: string; agent: SessionView["agent"] } | null = null,
  ): void {
    const now = Clock.now();
    const { width, zeca, vults, skyTop, flights, reflow } = this.layout;
    const fly = flights && !calm();

    if (zeca) {
      const who = focus ?? talking;
      if (!who) this.zeca = null;
      else {
        if (!this.zeca) {
          const bird = new Bird(ZECA, {
            x: zeca.x / zeca.scale,
            wireY: zeca.wire / zeca.scale,
            height: PERCH_HEIGHT,
            skyRight: width / zeca.scale,
            skyTop: 0,
          });
          if (fly) bird.arrive(now);
          this.zeca = { bird, key: "zeca", agent: who.agent, clip: "", idleSince: null, soaredAt: null, roosting: false };
        }
        this.zeca.agent = who.agent;
        this.want(this.zeca, focus ? clipFor(focus) : talking!.clip, now);
      }
    }

    const others = sessions.filter((s) => s !== focus).slice(0, vults.max);
    const present = new Set(others.map(key));
    for (const [k, v] of this.vults) {
      if (present.has(k) || v.leaving) continue;
      v.leaving = true;
      if (fly) v.bird.leave(now);
      else this.vults.delete(k);
    }
    others.forEach((s, index) => {
      const k = key(s);
      let v = this.vults.get(k);
      const slot = reflow ? index : v && !v.leaving ? v.slot : this.freeSlot();
      if (!v || v.leaving || v.slot !== slot) {
        const at = vults.at(slot);
        const bird = new Bird(ZECA, {
          x: at.x / vults.scale,
          wireY: at.wire / vults.scale,
          height: PERCH_HEIGHT,
          skyRight: width / vults.scale,
          skyTop,
        });
        // A vult moving up the list just sits in its new place; a new one flies in.
        if (fly && !v) bird.arrive(now);
        v = {
          bird,
          agent: s.agent,
          clip: "",
          idleSince: null,
          key: k,
          soaredAt: null,
          roosting: false,
          slot,
          leaving: false,
        };
        this.vults.set(k, v);
      }
      v.agent = s.agent;
      this.want(v, clipFor(s), now);
    });
    this.schedule(0);
  }

  private want(f: Flock, clip: string, now: number): void {
    if (clip === f.clip) return;
    f.clip = clip;
    f.idleSince = clip === "idle" ? now : null;
    f.soaredAt = null;
    f.roosting = false;
    // A soaring bird glides down to its perch first.
    f.bird.want(clip, now);
  }

  /** Any bird up in the thermal. */
  soaring(): boolean {
    return [this.zeca, ...this.vults.values()].some((f) => f?.bird.isSoaring());
  }

  /** The user called them: every soaring bird swoops down to its perch. True if any was up. */
  callDown(): boolean {
    const now = Clock.now();
    let any = false;
    for (const f of [this.zeca, ...this.vults.values()]) {
      if (!f?.bird.isSoaring()) continue;
      any = true;
      f.bird.call(now);
      f.soaredAt = null;
      // Down for a while before the next wait in the sky.
      f.idleSince = now;
    }
    if (any) this.schedule(0);
    return any;
  }

  /** This bird's lap of the sky's thermal, in its own cells. */
  private thermal(f: Flock, scale: number): Thermal {
    const sky = this.layout.sky!;
    const h = hash(f.key);
    return {
      cx: sky.cx / scale,
      cy: (sky.cy + ((h >> 3) % 3) - 1) / scale,
      rx: (sky.rx - (h % 5) * 7) / scale,
      ry: sky.ry / scale,
      lapMs: 9000 + (h % 4) * 700,
      phase: ((h % 360) * Math.PI) / 180,
    };
  }

  private freeSlot(): number {
    const used = new Set(
      [...this.vults.values()].filter((v) => !v.leaving).map((v) => v.slot),
    );
    let slot = 0;
    while (used.has(slot)) slot++;
    return slot;
  }

  private schedule(ms: number | null): void {
    window.clearTimeout(this.timer);
    this.timer =
      ms === null || !this.active
        ? undefined
        : window.setTimeout(() => this.draw(), ms);
  }

  private draw(): void {
    const now = Clock.now();
    const ctx = this.ctx;
    if (!this.colors) {
      const css = getComputedStyle(this.canvas);
      const v = (name: string, fallback: string) =>
        css.getPropertyValue(name).trim() || fallback;
      this.colors = {
        wire: v("--wire", "#3a3a40"),
        claude: v("--agent-claude", "#d97757"),
        codex: v("--agent-codex", "#19b48a"),
      };
    }
    const colors = this.colors;
    const accent = (agent: SessionView["agent"]) => ({ A: colors[agent] });
    const { wire, vults, zeca } = this.layout;
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    ctx.fillStyle = colors.wire;
    if (wire !== null)
      ctx.fillRect(0, wire * this.dpr, this.canvas.width, this.dpr);
    else {
      for (const v of this.vults.values()) {
        const at = vults.at(v.slot);
        ctx.fillRect(
          (at.x - 3) * this.dpr,
          at.wire * this.dpr,
          (BIRD_W * vults.scale + 6) * this.dpr,
          this.dpr,
        );
      }
    }

    let next = 1000;
    const soars = this.layout.sky !== null && !calm();
    /** What an idle bird does next: soar and roost under a sky, doze anywhere else. */
    const rest = (f: Flock, scale: number) => {
      if (f.idleSince === null || f.roosting) return;
      if (!soars) {
        if (now - f.idleSince > NAP_MS) {
          f.bird.want("sleep", now);
          f.roosting = true;
        } else next = Math.min(next, f.idleSince + NAP_MS - now + 1);
        return;
      }
      if (f.soaredAt === null) {
        if (now - f.idleSince >= SOAR_AFTER_MS) {
          f.bird.soar(this.thermal(f, scale), now);
          // Still landing from a flight: it tries again on the next frame.
          if (f.bird.isSoaring()) f.soaredAt = now;
        } else next = Math.min(next, f.idleSince + SOAR_AFTER_MS - now + 1);
      } else if (now - f.soaredAt >= SOAR_FOR_MS) {
        f.bird.want("sleep", now);
        f.soaredAt = null;
        f.roosting = true;
      }
    };

    for (const [k, v] of this.vults) {
      if (v.bird.gone(now)) {
        this.vults.delete(k);
        continue;
      }
      rest(v, vults.scale);
      const s = v.bird.shot(now);
      ctx.globalAlpha = s.alpha ?? 1;
      this.frames.draw(
        ctx,
        s.frame,
        s.x,
        s.y,
        vults.scale * this.dpr,
        s.flip,
        accent(v.agent),
      );
      next = Math.min(next, v.bird.nextChange(now));
    }
    if (this.zeca && zeca) {
      rest(this.zeca, zeca.scale);
      const z = this.zeca.bird.shot(now);
      ctx.globalAlpha = z.alpha ?? 1;
      this.frames.draw(
        ctx,
        z.frame,
        z.x,
        z.y,
        zeca.scale * this.dpr,
        z.flip,
        accent(this.zeca.agent),
      );
      next = Math.min(next, this.zeca.bird.nextChange(now));
    }
    ctx.globalAlpha = 1;
    // Nothing on the wire: one clear frame, then rest.
    if (this.zeca || this.vults.size) this.schedule(next);
  }
}

const key = (s: SessionView) => `${s.agent}:${s.id}`;
