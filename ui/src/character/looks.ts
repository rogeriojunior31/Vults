// Zeca's looks (a witch hat, a Santa hat, the drips…), drawn in design/mascots/zeca/zeca.py. A look
// is baked into a copy of a set's heads and flight frames, and what hangs from the neck becomes a
// layer under the band of every perched frame, so every clip, pose swap and frame cache works
// unchanged. The core picks which one is worn (crates/core/src/looks.rs). zeca.py's dress() is
// this function's twin for the review sheet: change both together.
import type { Frame, Grid, Layer, SpriteSet } from "./sprites";
import { ZECA } from "./zeca";

/** What hangs from the neck in one view: the strand's top-left from the band, and the pendant's from the strand. */
interface Neck {
  at: [number, number];
  strand: Grid;
  pat: [number, number] | null;
  pendant: Grid | null;
}

interface Look {
  parts: Record<string, Grid>;
  /** Pose (a head part's name, or "fly") → the look's part and its top-left on that part. */
  on: Record<string, [part: string, x: number, y: number]>;
  /** On the perched body ("side") and on the sunning pose ("front"); nothing in flight. */
  neck?: Record<"side" | "front", Neck>;
}

const LOOKS = (ZECA as SpriteSet & { looks?: Record<string, Look> }).looks ?? {};
export const LOOK_IDS = Object.keys(LOOKS);

/** Zeca's flight frames are 37 cells wide; a bigger species' grow on both sides. */
const ZECA_FLY_W = 37;
const FLIGHT = /^(fly_up|fly_down|glide(_v[lr]?)?)$/;
const width = (g: Grid) => Math.max(0, ...g.map((r) => r.length));

/** The first "E" in reading order: the eye in profile, the left eye facing you on head_front.
 * No species draws an E anywhere but the eye. */
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
  // frames by their own. The species' eye shift counts, so a look never starts above the grid.
  const poses = Object.keys(look.on).filter((p) => p !== "fly");
  const dy = (p: string) => eyeShift(set, p === "fly" ? "glide" : p)[1];
  const grow = (names: string[]) => Math.max(0, ...names.filter((p) => p in look.on).map((p) => -(look.on[p][2] + dy(p))));
  const lift = { head: grow(poses), fly: grow(["fly"]) };
  const parts: Record<string, Grid> = { ...set.parts };
  const shift = new Map<string, number>();
  for (const [name, grid] of Object.entries(set.parts)) {
    const fly = FLIGHT.test(name);
    if (!fly && !name.startsWith("head")) continue;
    // The longest pose its name starts with: head_down:blink and head_downR are head_down. A look
    // that is only a chain has no poses at all.
    const pose = fly ? (look.on.fly ? "fly" : undefined) : poses.filter((p) => name.startsWith(p)).sort((a, b) => b.length - a.length)[0];
    const up = fly ? lift.fly : lift.head;
    const w = width(grid);
    const out = [...Array<string>(up).fill(".".repeat(w)), ...grid.map((r) => r.padEnd(w, "."))].map((r) => [...r]);
    if (pose) {
      const [part, ax, ay] = look.on[pose];
      // Anchored on Zeca's head, a look follows this species' eye (the condor's comb adds a row);
      // an eye left of Zeca's clamps x at 0, as zeca.py does for a leaning pose: the look slides,
      // nothing is cut. A wider flight frame grows on both sides, which the centring covers: the
      // eye's shift is what is left over.
      const centring = (cols: number) => Math.floor((cols - ZECA_FLY_W) / 2);
      const [ex, ey] = eyeShift(set, fly ? "glide" : pose);
      const x0 = fly ? ax + ex + centring(w) - centring(width(set.parts.glide ?? [])) : Math.max(0, ax + ex);
      look.parts[part].forEach((row, j) =>
        [...row].forEach((c, i) => {
          const x = x0 + i, y = ay + ey + up + j;
          // The lift keeps every row; flight's x is not clamped, and a look may run past a part's bottom.
          if (c === "." || x < 0 || y >= out.length) return;
          while (out[y].length <= x) out[y].push(".");
          out[y][x] = c;
        }),
      );
    }
    parts[name] = out.map((r) => r.join(""));
    shift.set(name, up);
  }
  // What hangs from the neck is a body layer: it moves with no head, so a pose swap needs nothing.
  for (const [view, neck] of Object.entries(look.neck ?? {})) {
    parts[`strand_${view}`] = neck.strand;
    if (neck.pendant) parts[`pendant_${view}`] = neck.pendant;
  }
  const worn = (f: Frame): Layer[] => {
    // Grown parts sit higher by as much, so the bird stays where it was.
    const layers = f.layers.map(([p, x, y]): Layer => [p, x, y - (shift.get(p) ?? 0)]);
    const band = layers.findIndex(([p]) => p === "band");
    if (!look.neck || band < 0) return layers;
    // Hung from the band, under it (the agent's mark stays whole) and under the head, which covers
    // it when lowered. The front view on the sunning pose; flight has no band, so nothing there.
    const view = layers.some(([p]) => p === "sunning") ? "front" : "side";
    const { at, pat, pendant } = look.neck[view];
    const [, bx, by] = layers[band];
    const [x, y] = [bx + at[0], by + at[1]];
    const chain: Layer[] = [[`strand_${view}`, x, y]];
    // The pendant lags a cell behind a forward sway; the chest holds it on the way back.
    if (pendant && pat) chain.push([`pendant_${view}`, x + pat[0] - (f.dx > 0 ? 1 : 0), y + pat[1]]);
    layers.splice(band, 0, ...chain);
    return layers;
  };
  const clips = Object.fromEntries(
    Object.entries(set.clips).map(([name, clip]) => [name, { ...clip, frames: clip.frames.map((f) => ({ ...f, layers: worn(f) })) }]),
  );
  const out = { ...set, parts, clips } as T;
  byId.set(id, out);
  return out;
}

/** How Settings and the island's picker name and group the looks (docs/ANIMATIONS.md, Looks):
 *  the calendar's choice and none, the four seasonal looks, the ones worn on the head only, and
 *  the ones with something hanging from the neck. Ids are `crates/core/src/looks.rs`'s `Outfit`. */
export const LOOK_GROUPS: { title: string; looks: { value: string; label: string }[] }[] = [
  { title: "Calendar", looks: [{ value: "auto", label: "Auto (the calendar)" }, { value: "none", label: "None" }] },
  {
    title: "Seasonal",
    looks: [
      { value: "witch-hat", label: "Witch hat" },
      { value: "santa-hat", label: "Santa hat" },
      { value: "party-hat", label: "Party hat" },
      { value: "bunny-ears", label: "Bunny ears" },
    ],
  },
  {
    title: "Head",
    looks: [
      { value: "sunglasses", label: "Sunglasses" },
      { value: "west-coast", label: "West coast bandana" },
      { value: "fitted-cap", label: "Fitted cap" },
      { value: "mountain-hat", label: "Mountain hat" },
      { value: "headband", label: "Headband" },
      { value: "dreads", label: "Dreads and grill" },
    ],
  },
  {
    title: "With a chain",
    looks: [
      { value: "front-knot", label: "Red bandana, front knot" },
      { value: "durag", label: "Durag and grill" },
      { value: "crown", label: "Crown and chain" },
      { value: "bucket-hat", label: "Bucket hat and rope" },
      { value: "clock-chain", label: "Clock chain" },
      { value: "headphones", label: "Headphones" },
      { value: "shutter-shades", label: "Shutter shades" },
      { value: "chrome-chain", label: "Chrome chain" },
      { value: "eye-patch", label: "Eye patch and chains" },
    ],
  },
];
