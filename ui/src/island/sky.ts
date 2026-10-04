// One persistent bird per resting session (idle, or finished and waiting for the next prompt),
// shared by the compact and open island.
import { Bird, type Perch } from "../character/director";
import { FrameCache, clipLength, frameWidth, type SpriteSet } from "../character/sprites";
import { Clock } from "../clock";
import type { SessionView } from "../bridge";
import { clipFor } from "./behavior";
import { perchOf } from "../character/zeca";
import { SPECIES, speciesSet } from "../character/flock";
import { hash as hashOf, speciesOf } from "./flock";

export type SkyPerch = { x: number; y: number; scale: number };
/** The island's rectangle in the sky's coordinates, with its corner radius: the flock stays inside. */
export type SkyBox = { left: number; top: number; width: number; height: number; radius: number };
/** Room kept between a circling bird and the island's edge. */
const MARGIN = 14;
const key = (s: SessionView) => `${s.agent}:${s.id}`;
const IDLE_MS = 2000;
/** A finished session celebrates on its perch first (the done clip, with its species' signature),
 *  then takes off too: an agent waiting for the next prompt is resting, whether or not its "done"
 *  was dismissed. */
const DONE_MS = 6000;
/** About how long a new bird takes to fly in and land. */
const ARRIVAL_MS = 4500;
const resting = (s: SessionView) => s.status === "idle" || s.status === "finished";
const restAfter = (s: SessionView, set: SpriteSet) =>
  s.status === "finished" ? Math.max(DONE_MS, clipLength(set.clips.done) + 1000) : IDLE_MS;
/**
 * Each running subagent shows as a scout: a small Cathartes (the vultures that find food by smell)
 * taking off from its session's perch and circling low near it, gone when the subagent ends.
 */
const SCOUT_SPECIES = ["burrovianus", "aura", "melambrotus"];
const SCOUTS_PER_SESSION = 3;
const SCOUTS_MAX = 6;
/** The folded island is a thin pill with text across it: scouts fly only where there is room. */
const SCOUT_ROOM = 60;
/**
 * Now and then, while sessions are open, a vulture from outside the flock (a condor, a griffon)
 * glides in, rides the island's thermal once and goes on its way, never landing.
 */
const VISIT_EVERY_MS: [number, number] = [10 * 60_000, 20 * 60_000];
const VISIT_LAP_MS = 11_000;
const WIDTH = 720;
const HEIGHT = 560;

export class Sky {
  readonly canvas = document.createElement("canvas");
  private readonly ctx: CanvasRenderingContext2D;
  private readonly caches = new Map<SpriteSet, FrameCache>();
  private readonly birds = new Map<string, {
    bird: Bird; set: SpriteSet; session: SessionView; idleAt: number | null; airborne: boolean;
    landingAt: number | null; scale: number; leaving: boolean; hash: number;
    takeoffAt: number; launchScale: number;
  }>();
  /** By "<session key>#<n>": a scout for each of a session's running subagents. */
  private readonly scouts = new Map<string, { bird: Bird; set: SpriteSet; owner: string; index: number; leaving: boolean }>();
  private sessions: SessionView[] = [];
  private visitor: { bird: Bird; set: SpriteSet; leaveAt: number; leaving: boolean } | null = null;
  /** The user's choice in Settings → Flock; on by default. */
  private visitors = true;
  /** When the next visit is due; set once there are sessions to visit. */
  private nextVisit: number | null = null;
  private readonly dpr = Math.max(1, Math.round(devicePixelRatio || 1));
  private timer: number | undefined;
  private active = true;
  private readonly motion = matchMedia("(prefers-reduced-motion: reduce)");
  private anchors = new Map<string, SkyPerch>();
  private box: SkyBox = { left: 0, top: 0, width: WIDTH, height: 40, radius: 14 };
  private colors: Record<string, string> | null = null;

  constructor(private readonly changed: () => void) {
    this.canvas.className = "scene flock-sky";
    this.canvas.setAttribute("aria-hidden", "true");
    this.canvas.width = WIDTH * this.dpr;
    this.canvas.height = HEIGHT * this.dpr;
    this.ctx = this.canvas.getContext("2d")!;
    this.motion.addEventListener("change", () => {
      this.updateScouts(Clock.now());
      this.draw();
    });
  }

  owns(id: string): boolean { return this.birds.get(id)?.airborne ?? false; }

  /** Whether a visitor is in the sky (the tests watch it). */
  visiting(): boolean { return this.visitor !== null; }

  setVisitors(on: boolean): void {
    this.visitors = on;
    if (!on && this.visitor && !this.visitor.leaving) {
      this.visitor.leaving = true;
      this.visitor.bird.leave(Clock.now());
    }
  }

  /** A visitor now (the lab's `?visitor=1`, the tests); the next one comes on schedule. */
  visit(now: number): void {
    if (this.visitor) return;
    // A species nobody on the wire is, the rarer the better: the Old World's and the condors.
    const present = new Set(this.sessions.map((s) => s.species));
    const rare = SPECIES.filter((s) => (s.family === "old-world" || s.tall) && !present.has(s.id));
    const pick = rare[Math.floor(Math.random() * rare.length)] ?? SPECIES[0];
    const set = speciesSet(pick.id);
    const b = this.box;
    // It comes in from beyond the island's right edge, high.
    const perch = { x: b.left + b.width + 30, wireY: b.top - 10, height: perchOf(set), skyRight: b.left + b.width, skyTop: b.top + 4 };
    this.visitor = { bird: new Bird(set, perch), set, leaveAt: now + VISIT_LAP_MS, leaving: false };
    this.draw();
  }

  private scheduleVisit(now: number): void {
    const [low, high] = VISIT_EVERY_MS;
    this.nextVisit = now + low + Math.random() * (high - low);
  }

  /** Scouts circling (`live`), and all of them with those still flying off (the tests count them). */
  scouting(): { live: number; all: number } {
    return { live: [...this.scouts.values()].filter((s) => !s.leaving).length, all: this.scouts.size };
  }

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
      const set = speciesOf(id);
      // A bird on its perch takes its session's species at once (it just became Zeca, or king).
      if (f && !f.leaving && !f.airborne && f.set !== set) {
        f.set = set;
        f.bird = new Bird(set, this.perch(this.anchors.get(id) ?? { x: WIDTH / 2, y: 22, scale: 1 }, set));
        // The scene starts the new bird's clip over: its rest starts over too.
        if (f.idleAt !== null) f.idleAt = now;
      }
      const arriving = !f || f.leaving;
      if (!f || f.leaving) {
        const anchor = this.anchors.get(id) ?? { x: WIDTH / 2, y: 22, scale: 1 };
        const hash = hashOf(id);
        f = { bird: new Bird(set, this.perch(anchor, set)), set, session, idleAt: null,
          airborne: false, landingAt: null, scale: anchor.scale, leaving: false, hash,
          takeoffAt: now, launchScale: anchor.scale };
        this.birds.set(id, f);
      }
      const statusChanged = f.session.status !== session.status;
      f.session = session;
      if (resting(session)) {
        if (f.idleAt === null || statusChanged) {
          // A session that shows up finished flies in first: its done clip starts once it lands.
          f.idleAt = arriving && session.status === "finished" ? now + ARRIVAL_MS : now;
          // A finished bird keeps its done clip until it takes off; the scene plays it.
          if (session.status === "idle" && !f.bird.isSoaring()) f.bird.want("idle", now);
        }
      } else {
        f.idleAt = null;
        if (f.bird.isSoaring()) {
          // Keep web work on its perch after landing; the scene owns its next sortie.
          f.bird.want(clipFor(session) === "fly" ? "idle" : clipFor(session), now);
          f.landingAt = now;
        }
      }
    }
    this.sessions = sessions;
    if (sessions.length === 0) this.nextVisit = null;
    else if (this.nextVisit === null) this.scheduleVisit(now);
    this.updateScouts(now);
    this.draw();
  }

  /** Sends out and calls back scouts to match the sessions; true when one was sent out. */
  private updateScouts(now: number): boolean {
    const wanted = new Map<string, { owner: string; index: number }>();
    let sent = false;
    const calm = this.motion.matches || document.body.classList.contains("still");
    let total = 0;
    for (const s of calm || this.box.height < SCOUT_ROOM ? [] : this.sessions) {
      const n = Math.min(SCOUTS_PER_SESSION, s.subagents, SCOUTS_MAX - total);
      total += n;
      for (let index = 0; index < n; index++) wanted.set(`${key(s)}#${index}`, { owner: key(s), index });
    }
    for (const [id, scout] of this.scouts) {
      if (wanted.has(id) || scout.leaving) continue;
      scout.leaving = true;
      scout.bird.leave(now);
    }
    for (const [id, { owner, index }] of wanted) {
      if (this.scouts.get(id)?.leaving === false) continue;
      const set = speciesSet(SCOUT_SPECIES[hashOf(id) % SCOUT_SPECIES.length]);
      const bird = new Bird(set, this.perch(this.anchors.get(owner) ?? { x: WIDTH / 2, y: 22, scale: 1 }, set));
      this.scouts.set(id, { bird, set, owner, index, leaving: false });
      sent = true;
    }
    return sent;
  }

  place(anchors: Map<string, SkyPerch>, box: SkyBox): void {
    this.anchors = anchors;
    this.box = box;
    const now = Clock.now();
    for (const [id, f] of this.birds) {
      const at = anchors.get(id);
      if (at) {
        f.scale = at.scale;
        f.bird.movePerch(this.perch(at, f.set), now);
      }
    }
    // Scouts follow their session's perch and the island's edges too, and fold away with it.
    for (const scout of this.scouts.values()) {
      const at = anchors.get(scout.owner);
      if (at) scout.bird.movePerch(this.perch(at, scout.set), now);
    }
    // place() runs on every frame of a layout change: draw only when a scout was just sent out.
    if (this.updateScouts(now)) this.draw();
  }

  private perch(at: SkyPerch, set: SpriteSet): Perch {
    // Take-offs and arrivals turn at the island's own edges, not the screen's.
    const height = perchOf(set);
    return { x: at.x - 10, wireY: at.y + height - 6,
      height, skyRight: this.box.left + this.box.width, skyTop: this.box.top + 4 };
  }

  private cache(set: SpriteSet): FrameCache {
    let cache = this.caches.get(set);
    if (!cache) this.caches.set(set, (cache = new FrameCache(set)));
    return cache;
  }

  private draw(): void {
    clearTimeout(this.timer);
    const ctx = this.ctx, now = Clock.now();
    ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    const calm = this.motion.matches || document.body.classList.contains("still");
    let changed = false, next = 1000;
    if (!this.colors) {
      const css = getComputedStyle(this.canvas);
      this.colors = Object.fromEntries(["claude", "codex", "gemini", "other"].map(agent =>
        [agent, css.getPropertyValue(`--agent-${agent}`).trim()]));
    }
    // Nothing is drawn outside the island, whatever a flight's path.
    const b = this.box;
    ctx.save();
    ctx.beginPath();
    ctx.roundRect(b.left * this.dpr, b.top * this.dpr, b.width * this.dpr, b.height * this.dpr,
      [0, 0, b.radius * this.dpr, b.radius * this.dpr]);
    ctx.clip();
    for (const [id, f] of this.birds) {
      if (calm) {
        if (f.airborne) { f.airborne = false; changed = true; }
        f.bird = new Bird(f.set, this.perch(this.anchors.get(id) ?? { x: 360, y: 22, scale: 1 }, f.set));
        continue;
      }
      if (!this.active) continue;
      if (f.leaving && f.bird.gone(now)) { this.birds.delete(id); changed = true; continue; }
      if (!f.leaving && f.idleAt !== null && ((!f.airborne && now - f.idleAt >= restAfter(f.session, f.set)) || f.bird.isSoaring())) {
        const hash = f.hash;
        // Its species may have changed while it was up (it became king): take off as the new one.
        const set = speciesOf(id);
        if (!f.airborne && set !== f.set) {
          f.set = set;
          f.bird = new Bird(set, this.perch(this.anchors.get(id) ?? { x: WIDTH / 2, y: 22, scale: 1 }, set));
        }
        // Circle inside the island: separate phases and nested orbits keep an idle flock from
        // moving in lockstep. A folded island is a thin band, so its orbit is a flat oval.
        const rx = Math.max(20, b.width / 2 - MARGIN - 20 - hash % 5 * 16);
        const ry = Math.max(0, b.height / 2 - MARGIN - 8);
        f.bird.soar({ cx: b.left + b.width / 2, cy: b.top + b.height / 2 - 8 + (ry ? hash % 3 * 4 - 4 : 0),
          rx, ry, lapMs: 9000 + hash % 4 * 700, phase: hash % 360 * Math.PI / 180 }, now);
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
      const cx = flying ? frameWidth(f.set, shot.frame) / 2 : 10, cy = flying ? 8 : 6;
      ctx.save();
      ctx.translate((shot.x + cx) * this.dpr, (shot.y + cy) * this.dpr);
      ctx.scale(scale, scale);
      const accent = this.colors[f.session.agent];
      this.cache(f.set).drawShot(ctx, { ...shot, x: -cx, y: -cy }, this.dpr, accent ? { A: accent } : undefined);
      ctx.restore();
      next = Math.min(next, f.bird.nextChange(now));
    }
    for (const [id, scout] of this.scouts) {
      // No scouts with less motion: they would only stand in for the subagents count.
      if (calm) { this.scouts.delete(id); continue; }
      if (!this.active) continue;
      if (scout.leaving && scout.bird.gone(now)) { this.scouts.delete(id); continue; }
      if (!scout.leaving) {
        // Low and beside its session's bird (the lap starts past the bird, not over it), each scout
        // on its own small lap.
        const at = this.anchors.get(scout.owner) ?? { x: WIDTH / 2, y: 22, scale: 1 };
        const hash = hashOf(id), rx = 22 + scout.index * 10;
        scout.bird.soar({ cx: Math.min(at.x + 12 + rx, b.left + b.width - rx - MARGIN), cy: b.top + b.height - 16, rx,
          ry: b.height > 80 ? 6 : 0, lapMs: 4200 + hash % 5 * 300, phase: scout.index * 2.1 + hash % 360 * Math.PI / 180 }, now);
      }
      this.cache(scout.set).drawShot(ctx, scout.bird.shot(now), this.dpr);
      next = Math.min(next, scout.bird.nextChange(now));
    }
    next = Math.min(next, this.drawVisitor(ctx, now, calm));
    ctx.restore();
    if (changed) this.changed();
    if (this.active && !calm && (this.birds.size || this.scouts.size || this.visitor)) this.timer = window.setTimeout(() => this.draw(), next);
  }

  /** Brings in a visitor when one is due, and draws it; returns when it next changes. */
  private drawVisitor(ctx: CanvasRenderingContext2D, now: number, calm: boolean): number {
    if (calm) {
      this.visitor = null;
      return Infinity;
    }
    if (!this.active) return Infinity;
    if (!this.visitor && this.visitors && this.nextVisit !== null && now >= this.nextVisit) {
      this.scheduleVisit(now);
      this.visit(now);
    }
    const v = this.visitor;
    if (!v) return Infinity;
    if (v.leaving && v.bird.gone(now)) {
      this.visitor = null;
      return Infinity;
    }
    if (!v.leaving && now >= v.leaveAt) {
      v.leaving = true;
      v.bird.leave(now);
    }
    if (!v.leaving) {
      // One wide lap of the island's thermal, above the flock.
      const b = this.box;
      v.bird.soar({ cx: b.left + b.width / 2, cy: b.top + Math.max(10, b.height / 2 - 14), rx: Math.max(30, b.width / 2 - MARGIN - 30),
        ry: Math.max(0, b.height / 2 - MARGIN - 4), lapMs: VISIT_LAP_MS, phase: 0 }, now);
    }
    this.cache(v.set).drawShot(ctx, v.bird.shot(now), this.dpr);
    return v.bird.nextChange(now);
  }
}
