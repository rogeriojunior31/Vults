// Birds on a wire, in one canvas: Zeca (the focused session) and the vults (the other sessions).
// The island has three: the compact pill (everyone on one wire), the focus card (Zeca alone, on
// his piece of wire) and the flock list (one vult per row, each on a short perch). Birds arrive by
// flying in and leave by flying off where there is room; idle ones doze. A scene redraws only when
// a frame changes, and not at all while it is off screen. The shared sky owns idle flights. Timers,
// not requestAnimationFrame: WebKit pauses rAF while it believes the layer-shell surface is hidden.
import { Clock } from "../clock";
import type { SessionView } from "../bridge";
import { Bird, type Shot } from "../character/director";
import { FrameCache, frameAt, type Clip, type Frame, type SpriteSet } from "../character/sprites";
import { perchOf } from "../character/zeca";
import { clipFor, emoteFor } from "./behavior";
import { perchedSignature } from "../character/flock";
import { speciesOf, zecaSet } from "./flock";

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
  emotes: true,
};

/** No flights when the user asked for less motion (or the lab takes a still). */
const calm = () =>
  window.matchMedia("(prefers-reduced-motion: reduce)").matches ||
  document.body.classList.contains("still");
/** Zeca alone plays his signature this long after he starts resting, then this often. */
const SIGNATURE_FIRST_MS = 20_000;
const SIGNATURE_EVERY_MS = 60_000;
/** Where there is no sky (or no motion), an idle bird dozes off after this long. */
const NAP_MS = 90_000;

interface Flock {
  bird: Bird;
  /** Its species' sprites. */
  set: SpriteSet;
  /** Its session's key ("zeca" for Zeca): its place in the thermal comes from it. */
  key: string;
  agent: SessionView["agent"];
  clip: string;
  /** When it started idling, for the nap or the soaring. */
  idleSince: number | null;
  /** Back from soaring, dozing: it stays down until it has work. */
  roosting: boolean;
  /** The mark over its head for its state, if any. */
  emote: string | null;
  inSky?: boolean;
  /** Zeca alone on the wire plays his species' signature now and then, from this time on. */
  signatureAt?: number;
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
function extent(set: SpriteSet, frame: Frame): { w: number; h: number } {
  let w = 0;
  let h = 0;
  for (const [part, x, y] of frame.layers) {
    const grid = set.parts[part] ?? [];
    w = Math.max(w, x + Math.max(0, ...grid.map((r) => r.length)));
    h = Math.max(h, y + grid.length);
  }
  return { w, h };
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
  /** One frame cache per species: frames are cached by object, and each set has its own. */
  private readonly caches = new Map<SpriteSet, FrameCache>();
  /** Theme colors, read once: getComputedStyle on every frame is not free. */
  private colors: ({ wire: string } & Record<SessionView["agent"], string>) | null = null;
  /** Off screen: birds still follow their sessions, but nothing is drawn. */
  private active = true;

  constructor(layout: SceneLayout, private readonly hidden: (key: string) => boolean = () => false) {
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

  refresh(): void { this.schedule(0); }

  private cache(set: SpriteSet): FrameCache {
    let cache = this.caches.get(set);
    if (!cache) this.caches.set(set, (cache = new FrameCache(set)));
    return cache;
  }

  anchors(): { key: string; x: number; y: number; scale: number }[] {
    const { zeca, vults } = this.layout;
    const out = [];
    if (this.zeca && zeca) out.push({ key: this.zeca.key,
      x: zeca.x + 10 * zeca.scale, y: zeca.wire - (perchOf(this.zeca.set) - 6) * zeca.scale, scale: zeca.scale });
    for (const [key, v] of this.vults) {
      if (v.leaving) continue;
      const at = vults.at(v.slot);
      out.push({ key, x: at.x + 10 * vults.scale,
        y: at.wire - (perchOf(v.set) - 6) * vults.scale, scale: vults.scale });
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
        const set = zecaSet();
        const whose = focus ? key(focus) : "zeca";
        if (!this.zeca || this.zeca.key !== whose || this.zeca.set !== set) {
          const bird = new Bird(set, {
            x: zeca.x / zeca.scale,
            wireY: zeca.wire / zeca.scale,
            height: perchOf(set),
            skyRight: width / zeca.scale,
            skyTop: 0,
          });
          // A new species for the same Zeca (picked in the settings) changes in place: no fly-in.
          if (fly && this.zeca?.key !== whose) bird.arrive(now);
          this.zeca = { bird, set, key: whose, agent: who.agent, clip: "", idleSince: null, roosting: false, emote: null };
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
      const set = speciesOf(k);
      let v = this.vults.get(k);
      const slot = reflow ? index : v && !v.leaving ? v.slot : this.freeSlot();
      if (!v || v.leaving || v.slot !== slot || v.set !== set) {
        const at = vults.at(slot);
        const bird = new Bird(set, {
          x: at.x / vults.scale,
          wireY: at.wire / vults.scale,
          height: perchOf(set),
          skyRight: width / vults.scale,
          skyTop,
        });
        // A vult moving up the list just sits in its new place; a new one flies in.
        if (fly && !v) bird.arrive(now);
        v = {
          bird,
          set,
          agent: s.agent,
          clip: "",
          idleSince: null,
          key: k,
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
    f.signatureAt = clip === "idle" ? now + SIGNATURE_FIRST_MS : undefined;
    f.roosting = false;
    // A soaring bird glides down to its perch first.
    f.bird.want(clip, now);
  }

  /** Zeca's perched body, in CSS pixels within the canvas; null when he is not here. */
  zecaBox(): { x: number; y: number; w: number; h: number } | null {
    const z = this.layout.zeca;
    if (!this.zeca || !z) return null;
    const h = (perchOf(this.zeca.set) + 3) * z.scale;
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

  /**
   * Draws the mark over a bird's head for its state, centred over whatever head pose it is in
   * (in flight there is no head layer: no mark). Returns when it next changes.
   */
  private emote(f: Flock, shot: Shot, scale: number, now: number, top = -Infinity): number {
    const name = f.roosting ? "sleep" : f.emote;
    const clip = name && this.layout.emotes ? f.set.emotes?.[name] : undefined;
    const head = shot.frame.layers.find(([part]) => part.startsWith("head"));
    if (!clip || !head || shot.flip) return Infinity;
    const mark = frameAt(clip, now);
    const size = extent(f.set, mark);
    const headW = Math.max(0, ...(f.set.parts[head[0]] ?? []).map((r) => r.length));
    let x = shot.x + shot.frame.dx + head[1] + Math.round((headW - size.w) / 2) + 2;
    let y = shot.y + shot.frame.dy + head[2] - size.h - 1;
    // A tall bird in a list row has no room over its head: the mark goes beside it, in its own row.
    if (y < top) {
      // Beside the head, as far as the narrow list canvas allows.
      x = Math.min(shot.x + shot.frame.dx + head[1] + headW + 1, this.layout.width / scale - size.w);
      y = top;
    }
    this.ctx.globalAlpha = 1;
    this.cache(f.set).draw(this.ctx, mark, x, y, scale * this.dpr);
    return untilNextFrame(clip, now);
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
        gemini: v("--agent-gemini", "#4796e3"),
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
    const rest = (f: Flock) => {
      if (f.idleSince === null || f.roosting) return;
      if (now - f.idleSince > NAP_MS) {
        f.bird.want("sleep", now);
        f.roosting = true;
      } else next = Math.min(next, f.idleSince + NAP_MS - now + 1);
    };
    const inSky = (f: Flock) => {
      if (this.hidden(f.key)) { f.inSky = true; return true; }
      if (f.inSky) { f.inSky = false; f.roosting = false; f.bird.perchNow(f.clip, now); }
      return false;
    };

    for (const [k, v] of this.vults) {
      if (v.bird.gone(now)) {
        this.vults.delete(k);
        continue;
      }
      if (inSky(v)) continue;
      rest(v);
      const s = v.bird.shot(now);
      this.cache(v.set).drawShot(ctx, s, vults.scale * this.dpr, accent(v.agent));
      // In the list, a mark stays inside its own row.
      const top = this.layout.wire === null ? (vults.at(v.slot).wire - LIST_ROW + 2) / vults.scale : -Infinity;
      next = Math.min(next, this.emote(v, s, vults.scale, now, top));
      next = Math.min(next, v.bird.nextChange(now));
    }
    if (this.zeca && zeca && !inSky(this.zeca)) {
      rest(this.zeca);
      const z0 = this.zeca;
      // With nobody on the wire, Zeca is all there is to watch: now and then he does his own thing.
      if (z0.key === "zeca" && z0.signatureAt !== undefined && !z0.roosting && !calm() && perchedSignature(z0.set)) {
        if (now >= z0.signatureAt) {
          z0.signatureAt = now + SIGNATURE_EVERY_MS;
          z0.bird.react("signature", now);
        }
        next = Math.min(next, z0.signatureAt - now);
      }
      const z = this.zeca.bird.shot(now);
      this.cache(this.zeca.set).drawShot(ctx, z, zeca.scale * this.dpr, accent(this.zeca.agent));
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
  return clip === "think" ? "think" : clip === "question" ? "ask" : clip === "dance" ? "music" : null;
}
