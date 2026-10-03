// What a bird is doing over time. It receives what its session *wants* it to do and decides
// what it actually shows: clips hold for a minimum time so fast tool calls don't flicker,
// urgent states cut in at once, and a flight always ends with a landing before anything else.

import { clipLength, frameAt, frameWidth, type Frame, type SpriteSet } from "./sprites";

/** A clip plays at least this long before a non-urgent change (docs/ANIMATIONS.md). */
const MIN_MS = 600;
/** States a human must see now. */
const URGENT = new Set(["approval", "question", "fail"]);
/** Width of a species' flight frames, in cells: Zeca's 37, a condor's 73. */
const flyW = (set: SpriteSet) => Math.max(...set.parts.glide.map((r) => r.length));

/**
 * A flight frame by part name, made once per set (the frame cache keys on the object). A species
 * that glides with its wings in a V (the Cathartes) glides that way in the thermal too.
 */
const flightFrames = new WeakMap<SpriteSet, Map<string, Frame>>();
function flightFrame(set: SpriteSet, name: string): Frame {
  let frames = flightFrames.get(set);
  if (!frames) flightFrames.set(set, (frames = new Map()));
  let frame = frames.get(name);
  if (!frame) {
    const part = name === "glide" && set.parts.glide_v ? "glide_v" : name;
    frame = set.clips.fly.frames.find((f) => f.layers[0]?.[0] === part) ?? { ms: 0, dx: 0, dy: 0, layers: [[part, 0, 0]] };
    frames.set(name, frame);
  }
  return frame;
}

export interface Shot {
  frame: Frame;
  /** Top-left of the frame, in cells. */
  x: number;
  y: number;
  flip: boolean;
  /** Fainter on the far side of a thermal; 1 everywhere else. */
  alpha?: number;
  bank?: number;
  width?: number;
}

/**
 * A thermal to circle in, in cells: its centre, its radii (a circle seen from the side: wide and
 * flat), how long one lap takes, and where on it this bird is (radians), so a kettle of birds
 * spreads around it.
 */
export interface Thermal {
  cx: number;
  cy: number;
  rx: number;
  ry: number;
  lapMs: number;
  phase: number;
}

/** Taking off and climbing into the thermal. */
const CLIMB_MS = 1800;
/** A black vulture glides most of the time and flaps in short bursts: this often, this many. */
const FLAP_EVERY_MS = 3600;
const FLAPS = 3;
const FLAP_MS = 260;

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

type Point = { x: number; y: number };
const ZERO = { x: 0, y: 0 };
/** Hermite curve: preserve velocity where climbing and landing meet the orbit. */
function curve(a: Point, b: Point, va: Point, vb: Point, t: number, ms: number): Point {
  const u = Math.max(0, Math.min(1, t / ms)), u2 = u * u, u3 = u2 * u;
  const axis = (key: "x" | "y") => (2 * u3 - 3 * u2 + 1) * a[key]
    + (u3 - 2 * u2 + u) * ms * va[key]
    + (-2 * u3 + 3 * u2) * b[key] + (u3 - u2) * ms * vb[key];
  return { x: axis("x"), y: axis("y") };
}

function orbit(th: Thermal, t: number) {
  const a = th.phase + t / th.lapMs * Math.PI * 2;
  const omega = Math.PI * 2 / th.lapMs;
  return { x: th.cx + Math.cos(a) * th.rx, y: th.cy + Math.sin(a) * th.ry,
    vx: -Math.sin(a) * th.rx * omega, vy: Math.cos(a) * th.ry * omega, side: Math.sin(a) };
}

function bank(vx: number) {
  return { bank: Math.max(-0.10, Math.min(0.10, vx * 1.3)),
    width: 0.82 + 0.18 * Math.min(1, Math.abs(vx) / 0.045) };
}

const wingbeat = (t: number) => ["fly_up", "glide", "fly_down", "glide"][Math.floor(Math.max(0, t) / 65) % 4];

export class Bird {
  private clip = "idle";
  private since = 0;
  private wanted = "idle";
  private sortie: { start: number; legs: Leg[]; ms: number } | null = null;
  private leaving = false;
  /** Circling in a thermal, waiting for something to do, until told otherwise. */
  private soaring: { start: number; thermal: Thermal } | null = null;
  /** A clip played once (a startle, a hello) before going back to what is wanted. */
  private reacting = false;
  /** A reaction to play once the current flight lands. */
  private afterLanding: string | null = null;
  /** Where he looks while resting: at the pointer's side, or at you. */
  private look: "left" | "right" | "front" | null = null;
  /** A blink forced until then (the pointer arriving over him). */
  private blinkUntil = 0;

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
    if (this.soaring) {
      // Already up: just head off from where it is.
      const from = this.soarShot(now);
      this.soaring = null;
      this.flyLegs(departure(this.set, this.perch, from), now);
      return;
    }
    this.flyLegs(sortie(this.set, this.perch, "leave"), now);
  }

  /**
   * Take off and circle in `thermal` until something else is wanted. Called again while soaring,
   * it only moves the thermal. Ignored mid-flight (it can soar once landed).
   */
  soar(thermal: Thermal, now: number): void {
    if (this.leaving || this.sortie) return;
    if (this.soaring) {
      this.soaring.thermal = thermal;
      return;
    }
    this.soaring = { start: now, thermal };
    this.clip = "fly";
    this.since = now;
  }

  isSoaring(): boolean {
    return this.soaring !== null;
  }

  isFlying(): boolean {
    return this.soaring !== null || this.sortie !== null;
  }

  /** The shared sky completed this bird's landing; the local scene resumes on its perch. */
  perchNow(name: string, now: number): void {
    this.sortie = null;
    this.soaring = null;
    this.reacting = false;
    this.afterLanding = null;
    this.wanted = this.set.clips[name] ? name : "idle";
    this.start(this.wanted === "fly" ? "idle" : this.wanted, now);
  }

  /** A layout change moves the destination, preserving the bird's current flight position. */
  movePerch(perch: Perch, now: number): void {
    if (Object.keys(perch).every(k => perch[k as keyof Perch] === this.perch[k as keyof Perch])) return;
    const from = this.sortie ? this.shot(now) : null;
    const before = this.sortie ? this.sortieAt(Math.max(0, now - this.sortie.start - 1)) : null;
    Object.assign(this.perch, perch);
    if (from && before && this.sortie && !this.leaving) this.flyLegs(landing(this.set, this.perch, from, false,
      { x: from.x - before.x, y: from.y - before.y }), now);
  }

  /** Called down: a quick swoop onto the perch. */
  call(now: number): void {
    this.land(now, true);
  }

  private land(now: number, fast: boolean): void {
    if (!this.soaring) return;
    const from = this.soarShot(now);
    const before = this.soarShot(now - 1);
    const velocity = { x: from.x - before.x, y: from.y - before.y };
    this.soaring = null;
    this.flyLegs(landing(this.set, this.perch, from, fast, velocity), now);
  }

  gone(now: number): boolean {
    return this.leaving && (!this.sortie || now - this.sortie.start >= this.sortie.ms);
  }

  /**
   * Plays a clip once, then goes back to what is wanted. Never over a flight, nor over a state a
   * human must see (a permission, a question, a failure).
   */
  react(name: string, now: number): boolean {
    if (!this.set.clips[name] || this.sortie || this.soaring || this.leaving || URGENT.has(this.wanted)) return false;
    this.reacting = true;
    this.start(name, now);
    return true;
  }

  /** Comes down from above onto the perch, then plays `then` (a hello). */
  dropIn(now: number, then: string | null = null): void {
    const h = helpers(this.set, this.perch);
    const above: Shot = { frame: { ms: 0, dx: 0, dy: 0, layers: [] }, x: h.homeX + 6 - flyW(this.set) / 2, y: -10, flip: false };
    this.afterLanding = then;
    this.flyLegs(landing(this.set, this.perch, above, false), now);
  }

  /** Where to look while resting; null lets the idle clip look around on its own. */
  lookAt(look: "left" | "right" | "front" | null, now: number): void {
    if (look === "front" && this.look !== "front") this.blinkUntil = now + 140;
    this.look = look;
  }

  /** What the session wants now: a clip name, or "fly" for a sortie. A soaring bird lands first. */
  want(name: string, now: number): void {
    this.wanted = this.set.clips[name] ? name : "idle";
    if (this.soaring) this.land(now, URGENT.has(this.wanted));
    this.settle(now);
  }

  shot(now: number): Shot {
    this.settle(now);
    if (this.soaring) return this.soarShot(now);
    if (this.sortie) return this.sortieAt(now - this.sortie.start);
    let frame = frameAt(this.set.clips[this.clip], now - this.since);
    if (this.clip === "idle") frame = this.looking(frame, now);
    return { frame, x: this.perch.x, y: this.perch.wireY - this.perch.height, flip: false };
  }

  /** The resting head turned where he looks: at you (front) or over his shoulder (left). */
  private looking(frame: Frame, now: number): Frame {
    const resting = frame.layers.findIndex(([part]) => part === "head" || part === "head:blink");
    if (resting < 0 || (!this.look && now >= this.blinkUntil)) return frame;
    const blink = now < this.blinkUntil || frame.layers[resting][0].endsWith(":blink");
    const pose = this.look === "front" ? "head_front" : this.look === "left" ? "head_back" : "head";
    // Offsets from the side head's socket, as the idle and hello clips place these heads.
    const [dx, dy] = this.look === "front" ? [-2, -1] : this.look === "left" ? [-5, -1] : [0, 0];
    const [, x, y] = frame.layers[resting];
    const layers = frame.layers.slice();
    layers[resting] = [blink ? `${pose}:blink` : pose, x + dx, y + dy];
    return { ...frame, layers };
  }

  /** Milliseconds until the picture changes, to schedule the next draw. */
  nextChange(now: number): number {
    if (this.soaring || this.sortie) return 1000 / 30;
    if (now < this.blinkUntil) return this.blinkUntil - now;
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
    // Soaring lasts until `want` or `call` brings it down.
    if (this.soaring) return;
    if (this.sortie) {
      if (now - this.sortie.start < this.sortie.ms) return;
      // Landed (or out of sight). Fly again only if that is still the job.
      if (this.leaving) return;
      this.sortie = null;
      this.start(this.wanted === "fly" ? "idle" : this.wanted, now);
      if (this.wanted === "fly") this.takeOff(now);
      else if (this.afterLanding) this.react(this.afterLanding, now);
      this.afterLanding = null;
      return;
    }
    if (this.reacting) {
      const clip = this.set.clips[this.clip];
      // A state a human must see cuts in; anything else waits for the reaction to end.
      if (now - this.since < clipLength(clip) && !URGENT.has(this.wanted)) return;
      this.reacting = false;
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

  /** Where the soaring bird is: climbing out, then round and round the thermal. */
  private soarShot(now: number): Shot {
    const { start, thermal: th } = this.soaring!;
    const t = now - start;
    const h = helpers(this.set, this.perch);
    const entry = orbit(th, 0);
    if (t < 180) return h.perchedShot(t, 1);
    if (t < CLIMB_MS) {
      const pos = (time: number) => curve({ x: h.homeX, y: h.landY }, entry, ZERO,
        { x: entry.vx, y: entry.vy }, time - 180, CLIMB_MS - 180);
      const p = pos(t), before = pos(t - 1);
      const vx = p.x - before.x;
      return { ...h.at(h.one(wingbeat(t - 180)), p.x, p.y, vx < 0), ...bank(vx) };
    }
    const p = orbit(th, t - CLIMB_MS);
    // Short bursts of flaps between long glides, each bird on its own beat.
    const beat = ((t + th.phase * 1000) % FLAP_EVERY_MS + FLAP_EVERY_MS) % FLAP_EVERY_MS;
    const frame = beat < FLAPS * FLAP_MS ? wingbeat(beat) : "glide";
    return {
      ...h.at(h.one(frame), p.x, p.y, p.vx < 0), ...bank(p.vx),
      // The far half of the lap (the upper one) is further away: fainter.
      alpha: 0.8 + p.side * 0.2,
    };
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
/** Placing a bird: perched (facing either way) or in flight (from its centre). */
function helpers(set: SpriteSet, p: Perch) {
  const perched = set.clips.idle;
  const one = (name: string): Frame => flightFrame(set, name);
  const W = flyW(set);
  // Facing left keeps the body where it was: mirror around the body's centre (10 cells in).
  const perchedShot = (t: number, dy: number, flip = false): Shot => {
    const f = frameAt(perched, t);
    const x = flip ? p.x + 21 - frameWidth(set, f) : p.x;
    return { frame: { ...f, dy: f.dy + dy }, x, y: p.wireY - p.height, flip };
  };
  // Flight frames are placed from the bird's centre.
  const at = (frame: Frame, cx: number, cy: number, flip: boolean): Shot => ({
    frame,
    x: cx - W / 2,
    y: cy - 8,
    flip,
  });
  // The perched body is centred about 10 cells in; flight frames on their middle column. A flight
  // frame's centre must be at `landY` for its body to match the perched body.
  return { one, perchedShot, at, W, homeX: p.x + 10, landY: p.wireY - p.height + 6 };
}

/** The centre of a flight frame, from the shot that drew it. */
const centre = (s: Shot, w: number) => ({ x: s.x + w / 2, y: s.y + 8 });

/**
 * Down from wherever it is in the sky: a glide home, wings up to brake, touch down. `fast` is a
 * bird called down by the user: a swoop, no looking around.
 */
function landing(set: SpriteSet, p: Perch, from: Shot, fast: boolean, velocity = ZERO): Leg[] {
  const h = helpers(set, p);
  const start = centre(from, h.W);
  const left = start.x > h.homeX;
  const glideMs = fast ? 380 : Math.min(Math.max(600, Math.abs(start.x - h.homeX) * 22), 2400);
  const legs: Leg[] = [
    {
      ms: glideMs,
      shot: (t) => {
        const pos = (time: number) => curve(start, { x: h.homeX, y: h.landY - 3 }, velocity, ZERO, time, glideMs);
        const at = pos(t), before = pos(t - 1);
        const vx = t === 0 ? velocity.x : at.x - before.x;
        return { ...h.at(h.one("glide"), at.x, at.y, Math.abs(vx) < 0.00001 ? left : vx < 0),
          ...bank(vx), alpha: lerp(from.alpha ?? 1, 1, t / glideMs) };
      },
    },
    {
      ms: fast ? 200 : 360,
      shot: (t) => h.at(h.one(t < (fast ? 130 : 240) ? "fly_up" : "fly_down"), h.homeX, lerp(h.landY - 3, h.landY, t / (fast ? 200 : 360)), left),
    },
    { ms: 140, shot: (t) => h.perchedShot(t, 1, left) },
  ];
  if (!fast && left) {
    // Look around, then hop to face the island again.
    legs.push(
      { ms: 500, shot: (t) => h.perchedShot(t, 0, true) },
      { ms: 120, shot: (t) => h.perchedShot(t, -2, true) },
      { ms: 120, shot: (t) => h.perchedShot(t, -1) },
    );
  }
  return legs;
}

/** Off past the right edge, from wherever it is in the sky. */
function departure(set: SpriteSet, p: Perch, from: Shot): Leg[] {
  const h = helpers(set, p);
  const start = centre(from, h.W);
  const offX = p.skyRight + h.W;
  const ms = Math.min(Math.max(400, (offX - start.x) * 22), 2400);
  return [{ ms, shot: (t) => h.at(frameAt(set.clips.fly, t), lerp(start.x, offX, t / ms), start.y, false) }];
}

function sortie(set: SpriteSet, p: Perch, mode: Mode = "round"): Leg[] {
  const fly = set.clips.fly;
  const { one, perchedShot, at, homeX, W } = helpers(set, p);
  const cruiseY = p.skyTop + 8;
  const offX = p.skyRight + W;
  const farX = mode === "round" ? p.skyRight - W / 2 : offX;
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
        const beat = wingbeat(t);
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
