// Birds on a wire, in one canvas: Zeca (the focused session) and the vults (the other sessions).
// The island has three: the compact pill (everyone on one wire), the focus card (Zeca alone, on
// his piece of wire) and the flock list (one vult per row, each on a short perch). Birds arrive by
// flying in and leave by flying off where there is room; idle ones doze. A scene redraws only when
// a frame changes, and not at all while it is off screen, so a quiet island costs no CPU. Timers,
// not requestAnimationFrame: WebKit pauses rAF while it believes the layer-shell surface is hidden.
import { Clock } from "../clock";
import type { SessionView } from "../bridge";
import { Bird, type Shot, type Thermal } from "../character/director";
import { FrameCache, frameAt, type Clip, type Frame } from "../character/sprites";
import { PERCH_HEIGHT, ZECA } from "../character/zeca";
import { clipFor, emoteFor } from "./behavior";

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
  /** A mark over each bird's head for its state (a bang, a thought bubble); off where it won't fit. */
  emotes: boolean;
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
  // No room over their heads: the badges say it.
  emotes: false,
};

/** The focus card: Zeca large, on his own piece of wire. */
/** Zeca large (3x) on his own piece of wire, with room over his head for a mark. */
export const FOCUS_W = 112;
export const FOCUS_H = 108;
export const FOCUS_SCENE: SceneLayout = {
  width: FOCUS_W,
  height: FOCUS_H,
  wire: 96,
  zeca: { x: (FOCUS_W - 3 * 24) / 2, wire: 96, scale: 3 },
  vults: { scale: 1, max: 0, at: () => ({ x: 0, wire: 0 }) },
  skyTop: 0,
  flights: false,
  reflow: false,
  sky: null,
  emotes: true,
};

/** The flock list: one vult per row. */
export const LIST_ROW = 32;
export const LIST_SCENE: SceneLayout = {
  width: 34,
  height: LIST_ROW,
  wire: null,
  zeca: null,
  vults: {
    scale: 1,
    max: 32,
    // Low in the row, so a mark fits over the head.
    at: (slot) => ({ x: 3, wire: slot * LIST_ROW + 30 }),
  },
  skyTop: 0,
  flights: false,
  reflow: true,
  sky: null,
  emotes: true,
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
  /** The mark over its head for its state, if any. */
  emote: string | null;
}

/** Milliseconds until a looping clip shows its next frame. */
function untilNextFrame(clip: Clip, t: number): number {
  const total = clip.frames.reduce((a, f) => a + f.ms, 0);
  let left = t % total;
  for (const f of clip.frames) {
    if (left < f.ms) return Math.max(16, f.ms - left);
    left -= f.ms;
  }
  return 100;
}

/** How far a frame's parts reach, in cells. */
function extent(set: typeof ZECA, frame: Frame): { w: number; h: number } {
  let w = 0;
  let h = 0;
  for (const [part, x, y] of frame.layers) {
    const grid = set.parts[part] ?? [];
    w = Math.max(w, x + Math.max(0, ...grid.map((r) => r.length)));
    h = Math.max(h, y + grid.length);
  }
  return { w, h };
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
  private colors: { wire: string; claude: string; codex: string; other: string } | null = null;
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
          this.zeca = { bird, key: "zeca", agent: who.agent, clip: "", idleSince: null, soaredAt: null, roosting: false, emote: null };
        }
        this.zeca.agent = who.agent;
        this.want(this.zeca, focus ? clipFor(focus) : talking!.clip, now);
        this.zeca.emote = focus ? emoteFor(focus) : talkingEmote(talking!.clip);
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
          emote: null,
          slot,
          leaving: false,
        };
        this.vults.set(k, v);
      }
      v.agent = s.agent;
      this.want(v, clipFor(s), now);
      v.emote = emoteFor(s);
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

  /** Zeca's perched body, in CSS pixels within the canvas; null when he is not here. */
  zecaBox(): { x: number; y: number; w: number; h: number } | null {
    const z = this.layout.zeca;
    if (!this.zeca || !z) return null;
    const h = (PERCH_HEIGHT + 3) * z.scale;
    return { x: z.x, y: z.wire - h + 3 * z.scale, w: BIRD_W * z.scale, h };
  }

  /** Where Zeca looks while resting (the pointer's side, or at you). */
  lookAt(look: "left" | "right" | "front" | null): void {
    this.zeca?.bird.lookAt(look, Clock.now());
    this.schedule(0);
  }

  /** Zeca plays a clip once (a startle, a preen); false when he can't now. */
  react(name: string): boolean {
    const ok = this.zeca?.bird.react(name, Clock.now()) ?? false;
    if (ok) this.schedule(0);
    return ok;
  }

  /** Zeca comes down onto his perch from above, then plays `then`. */
  dropIn(then: string | null): void {
    if (!this.zeca || calm()) return;
    this.zeca.bird.dropIn(Clock.now(), then);
    this.schedule(0);
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

  /**
   * Draws the mark over a bird's head for its state, centred over whatever head pose it is in
   * (in flight there is no head layer: no mark). Returns when it next changes.
   */
  private emote(f: Flock, shot: Shot, scale: number, now: number): number {
    const name = f.roosting ? "sleep" : f.emote;
    const clip = name && this.layout.emotes ? ZECA.emotes?.[name] : undefined;
    const head = shot.frame.layers.find(([part]) => part.startsWith("head"));
    if (!clip || !head || shot.flip) return Infinity;
    const mark = frameAt(clip, now);
    const size = extent(ZECA, mark);
    const headW = Math.max(0, ...(ZECA.parts[head[0]] ?? []).map((r) => r.length));
    const x = shot.x + shot.frame.dx + head[1] + Math.round((headW - size.w) / 2) + 2;
    const y = shot.y + shot.frame.dy + head[2] - size.h - 1;
    this.ctx.globalAlpha = 1;
    this.frames.draw(this.ctx, mark, x, y, scale * this.dpr);
    return untilNextFrame(clip, now);
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
        other: v("--agent-other", "#a78bfa"),
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
      next = Math.min(next, this.emote(v, s, vults.scale, now));
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
      next = Math.min(next, this.emote(this.zeca, z, zeca.scale, now));
      next = Math.min(next, this.zeca.bird.nextChange(now));
    }
    ctx.globalAlpha = 1;
    // Nothing on the wire: one clear frame, then rest.
    if (this.zeca || this.vults.size) this.schedule(next);
  }
}

const key = (s: SessionView) => `${s.agent}:${s.id}`;

/** The mark over Zeca in the chat: thinking while it answers, a question while it asks. */
function talkingEmote(clip: string): string | null {
  return clip === "think" ? "think" : clip === "question" ? "ask" : null;
}
