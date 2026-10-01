// What a bird is doing over time. It receives what its session *wants* it to do and decides
// what it actually shows: clips hold for a minimum time so fast tool calls don't flicker,
// urgent states cut in at once, and a flight always ends with a landing before anything else.

import { clipLength, frameAt, frameWidth, type Frame, type SpriteSet } from "./sprites";

/** A clip plays at least this long before a non-urgent change (docs/ANIMATIONS.md). */
const MIN_MS = 600;
/** States a human must see now. */
const URGENT = new Set(["approval", "question", "fail"]);
/** Width of the flight frames, in cells. */
const FLY_W = 37;

export interface Shot {
  frame: Frame;
  /** Top-left of the frame, in cells. */
  x: number;
  y: number;
  flip: boolean;
}

export interface Perch {
  /** Where the body's top-left sits when perched, in cells. */
  x: number;
  /** The wire's row. */
  wireY: number;
  /** Rows between the body's top-left and the wire. */
  height: number;
  /** How far right the bird may fly, in cells. */
  skyRight: number;
  /** Highest row it may fly at. */
  skyTop: number;
  /** Circle in a thermal before heading home (needs room). */
  thermal?: boolean;
}

type Leg = { ms: number; shot: (t: number) => Shot };

const ease = (k: number) => k * k * (3 - 2 * k);
const lerp = (a: number, b: number, k: number) => a + (b - a) * k;

export class Bird {
  private clip = "idle";
  private since = 0;
  private wanted = "idle";
  private sortie: { start: number; legs: Leg[]; ms: number } | null = null;
  private leaving = false;

  constructor(
    private readonly set: SpriteSet,
    private readonly perch: Perch,
  ) {}

  /** Glide in from the right edge and land, instead of appearing. */
  arrive(now: number): void {
    this.flyLegs(sortie(this.set, this.perch, "arrive"), now);
  }

  /** Take off and fly out of the right edge; `gone` turns true when it is out of sight. */
  leave(now: number): void {
    this.leaving = true;
    this.flyLegs(sortie(this.set, this.perch, "leave"), now);
  }

  gone(now: number): boolean {
    return this.leaving && (!this.sortie || now - this.sortie.start >= this.sortie.ms);
  }

  /** What the session wants now: a clip name, or "fly" for a sortie. */
  want(name: string, now: number): void {
    this.wanted = this.set.clips[name] ? name : "idle";
    this.settle(now);
  }

  shot(now: number): Shot {
    this.settle(now);
    if (this.sortie) return this.sortieAt(now - this.sortie.start);
    const frame = frameAt(this.set.clips[this.clip], now - this.since);
    return { frame, x: this.perch.x, y: this.perch.wireY - this.perch.height, flip: false };
  }

  /** Milliseconds until the picture changes, to schedule the next draw. */
  nextChange(now: number): number {
    if (this.sortie) return 40;
    const clip = this.set.clips[this.clip];
    const total = clipLength(clip);
    const t = now - this.since;
    if (!clip.loop && t >= total) return MIN_MS;
    let left = clip.loop ? t % total : t;
    for (const f of clip.frames) {
      if (left < f.ms) return Math.max(16, f.ms - left);
      left -= f.ms;
    }
    return MIN_MS;
  }

  private settle(now: number): void {
    if (this.sortie) {
      if (now - this.sortie.start < this.sortie.ms) return;
      // Landed (or out of sight). Fly again only if that is still the job.
      if (this.leaving) return;
      this.sortie = null;
      this.start(this.wanted === "fly" ? "idle" : this.wanted, now);
      if (this.wanted === "fly") this.takeOff(now);
      return;
    }
    if (this.wanted === this.clip) return;
    if (now - this.since < MIN_MS && !URGENT.has(this.wanted)) return;
    if (this.wanted === "fly") this.takeOff(now);
    else this.start(this.wanted, now);
  }

  private start(clip: string, now: number): void {
    this.clip = clip;
    this.since = now;
  }

  private takeOff(now: number): void {
    this.flyLegs(sortie(this.set, this.perch), now);
  }

  private flyLegs(legs: Leg[], now: number): void {
    this.clip = "fly";
    this.since = now;
    this.sortie = { start: now, legs, ms: legs.reduce((s, l) => s + l.ms, 0) };
  }

  private sortieAt(t: number): Shot {
    const legs = this.sortie?.legs ?? [];
    let left = t;
    for (const leg of legs) {
      if (left < leg.ms) return leg.shot(left);
      left -= leg.ms;
    }
    return legs[legs.length - 1].shot(legs[legs.length - 1].ms);
  }
}

type Mode = "round" | "arrive" | "leave";

/**
 * A flight. "round": crouch, take off with deep beats, flap-and-glide out, turn (or ride a
 * thermal), glide home, flare, touch down, turn to face the island. "arrive": only the way in,
 * from beyond the right edge. "leave": only the way out, past the right edge.
 */
function sortie(set: SpriteSet, p: Perch, mode: Mode = "round"): Leg[] {
  const fly = set.clips.fly;
  const perched = set.clips.idle;
  const one = (name: string): Frame => ({ ms: 0, dx: 0, dy: 0, layers: [[name, 0, 0]] });
  // Facing left keeps the body where it was: mirror around the body's centre (10 cells in).
  const perchedShot = (t: number, dy: number, flip = false): Shot => {
    const f = frameAt(perched, t);
    const x = flip ? p.x + 21 - frameWidth(set, f) : p.x;
    return { frame: { ...f, dy: f.dy + dy }, x, y: p.wireY - p.height, flip };
  };
  // Flight frames are placed from the bird's centre.
  const at = (frame: Frame, cx: number, cy: number, flip: boolean): Shot => ({
    frame,
    x: Math.round(cx - FLY_W / 2),
    y: Math.round(cy - 8),
    flip,
  });

  // The perched body is centred about 10 cells in; flight frames on their middle column.
  const homeX = p.x + 10;
  const cruiseY = p.skyTop + 8;
  const offX = p.skyRight + FLY_W;
  const farX = mode === "round" ? p.skyRight - FLY_W / 2 : offX;
  const span = Math.max(10, farX - homeX);
  // About 45 cells a second, but never a long wait on a wide sky.
  const outMs = Math.min(span * 22, 2800);
  const homeMs = Math.min(span * 26, 3200);
  // Where a flight frame's centre must be for its body to match the perched body.
  const landY = p.wireY - p.height + 6;

  const out: Leg[] = [
    { ms: 180, shot: (t) => perchedShot(t, 1) },
    {
      ms: 700,
      shot: (t) => {
        const k = ease(t / 700);
        const beat = Math.floor(t / 140) % 2 === 0 ? "fly_down" : "fly_up";
        return at(one(beat), lerp(homeX, homeX + span * 0.2, k), lerp(p.wireY - 12, cruiseY, k), false);
      },
    },
    {
      ms: outMs,
      shot: (t) => at(frameAt(fly, t), lerp(homeX + span * 0.2, farX, t / outMs), cruiseY + Math.sin(t / 380) * 1.5, false),
    },
  ];
  const thermal: Leg[] = p.thermal
    ? [
        {
          ms: 4200,
          shot: (t) => {
            const a = (t / 4200) * Math.PI * 2;
            const r = Math.min(26, span / 3);
            return at(one("glide"), farX - r + Math.cos(a) * r, cruiseY - (t / 4200) * 6 + Math.sin(a) * 3, Math.sin(a) > 0);
          },
        },
      ]
    : [];
  const backFrom = cruiseY - (p.thermal && mode === "round" ? 6 : 0);
  const back: Leg[] = [
    {
      ms: homeMs,
      shot: (t) => {
        const k = ease(t / homeMs);
        const frame = t < homeMs * 0.3 ? frameAt(fly, t) : one("glide");
        return at(frame, lerp(farX, homeX, k), lerp(backFrom, landY - 3, k), true);
      },
    },
    // Wings up to brake, then a last beat as the feet touch.
    {
      ms: 360,
      shot: (t) => at(one(t < 240 ? "fly_up" : "fly_down"), homeX, lerp(landY - 3, landY, t / 360), true),
    },
    // Touch down facing the way it flew, look around, then hop to face the island again.
    { ms: 140, shot: (t) => perchedShot(t, 1, true) },
    { ms: 600, shot: (t) => perchedShot(t, 0, true) },
    { ms: 120, shot: (t) => perchedShot(t, -2, true) },
    { ms: 120, shot: (t) => perchedShot(t, -1) },
  ];
  if (mode === "arrive") return back;
  if (mode === "leave") return out;
  return [...out, ...thermal, ...back];
}
