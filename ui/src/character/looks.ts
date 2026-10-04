// Zeca's seasonal looks (a witch hat, a Santa hat…), drawn in design/mascots/zeca/zeca.py. A look
// is baked into a copy of a set's heads and flight frames, so every clip, pose swap and frame
// cache works unchanged. The core picks which one is worn (crates/core/src/looks.rs). zeca.py's
// dress() is this function's twin for the review sheet: change both together.
import type { Grid, Layer, SpriteSet } from "./sprites";
import { ZECA } from "./zeca";

interface Look {
  parts: Record<string, Grid>;
  /** Pose (a head part's name, or "fly") → the look's part and its top-left on that part. */
  on: Record<string, [part: string, x: number, y: number]>;
}

const LOOKS = (ZECA as SpriteSet & { looks?: Record<string, Look> }).looks ?? {};
export const LOOK_IDS = Object.keys(LOOKS);

/** Zeca's flight frames are 37 cells wide; a bigger species' grow on both sides. */
const ZECA_FLY_W = 37;
const FLIGHT = /^(fly_up|fly_down|glide(_v[lr]?)?)$/;
const width = (g: Grid) => Math.max(0, ...g.map((r) => r.length));

/** Where the eye is: the first "E" cell in reading order. */
const eye = (g: Grid): [number, number] | undefined => {
  for (let y = 0; y < g.length; y++) {
    const x = g[y].indexOf("E");
    if (x >= 0) return [x, y];
  }
  return undefined;
};
/** How far `set`'s eye on `part` sits from Zeca's; none on either part means no shift. */
function eyeShift(set: SpriteSet, part: string): [number, number] {
  const a = eye(set.parts[part] ?? []), z = eye(ZECA.parts[part] ?? []);
  return a && z ? [a[0] - z[0], a[1] - z[1]] : [0, 0];
}

const dressed = new WeakMap<SpriteSet, Map<string, SpriteSet>>();

/** `set` wearing the look `id`; the set itself for an unknown id or none. */
export function dress<T extends SpriteSet>(set: T, id: string | null): T {
  const look = id ? LOOKS[id] : undefined;
  if (!id || !look) return set;
  let byId = dressed.get(set);
  if (!byId) dressed.set(set, (byId = new Map()));
  const done = byId.get(id);
  if (done) return done as T;

  // Every head grows by the same rows (a clip swaps poses mid-way: the hat must not jump); flight
  // frames by their own.
  const poses = Object.keys(look.on).filter((p) => p !== "fly");
  const grow = (names: string[]) => Math.max(0, ...names.map((p) => -look.on[p][2]));
  const lift = { head: grow(poses), fly: grow(["fly"]) };
  const parts: Record<string, Grid> = { ...set.parts };
  const shift = new Map<string, number>();
  for (const [name, grid] of Object.entries(set.parts)) {
    const fly = FLIGHT.test(name);
    if (!fly && !name.startsWith("head")) continue;
    // The longest pose its name starts with: head_down:blink and head_downR are head_down.
    const pose = fly ? "fly" : poses.filter((p) => name.startsWith(p)).sort((a, b) => b.length - a.length)[0];
    const up = fly ? lift.fly : lift.head;
    const w = width(grid);
    const out = [...Array<string>(up).fill(".".repeat(w)), ...grid.map((r) => r.padEnd(w, "."))].map((r) => [...r]);
    if (pose) {
      const [part, ax, ay] = look.on[pose];
      // A look is anchored on Zeca's head: it follows this species' eye (the condor's comb adds a
      // row). A wider flight frame grows on both sides, which the centring covers: the eye's shift
      // is what is left over.
      const centring = (w: number) => (fly ? Math.floor((w - ZECA_FLY_W) / 2) : 0);
      const [ex, ey] = eyeShift(set, fly ? "glide" : pose);
      const x0 = ax + centring(w) + ex - centring(width(set.parts.glide ?? []));
      look.parts[part].forEach((row, j) =>
        [...row].forEach((c, i) => {
          const x = x0 + i, y = ay + ey + up + j;
          // A pose whose crown moved left may push a brim's edge off the grid.
          if (c === "." || x < 0 || y < 0 || y >= out.length) return;
          while (out[y].length <= x) out[y].push(".");
          out[y][x] = c;
        }),
      );
    }
    parts[name] = out.map((r) => r.join(""));
    shift.set(name, up);
  }
  // Grown parts sit higher by as much, so the bird stays where it was.
  const clips = Object.fromEntries(
    Object.entries(set.clips).map(([name, clip]) => [
      name,
      { ...clip, frames: clip.frames.map((f) => ({ ...f, layers: f.layers.map(([p, x, y]): Layer => [p, x, y - (shift.get(p) ?? 0)]) })) },
    ]),
  );
  const out = { ...set, parts, clips } as T;
  byId.set(id, out);
  return out;
}
