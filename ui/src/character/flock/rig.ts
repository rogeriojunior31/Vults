// Pixel transforms that turn Zeca's rig into another species: recolors, patches, a longer neck, a
// taller body, a bigger wingspan. Every transform keeps the anchors in step (head socket, band,
// feet), so all of Zeca's clips keep working on the new bird.
import type { Clip, Frame, Grid, Layer, SpriteSet } from "../sprites";

/** A species' sprite set while it is built, with where its feet sit. */
export interface Rig extends SpriteSet {
  /** Rows from the body's top-left down to the wire. */
  perch: number;
  /** Extra rows of the tall body (big vultures), if it has one. */
  tall?: number;
}

export const FLIGHT = ["fly_up", "glide", "fly_down"];
export const BODIES = ["body", "body_puff"];
/** The spread-wing pose and its settled twin (the approval clip breathes between them). */
export const SUNNING = ["sunning", "sunning_low"];

const partW = (g: Grid): number =>
  Math.max(0, ...g.map((r) => r.length));

export const recolor = (
  g: Grid,
  map: Record<string, string>,
  from = 0,
  to = Infinity,
): Grid =>
  g.map((r, i) =>
    i < from || i >= to ? r : [...r].map((c) => map[c] ?? c).join(""),
  );

export function patch(g: Grid, pts: [number, number][], ch: string): Grid {
  const a = g.map((r) => [...r]);
  for (const [x, y] of pts) {
    if (!a[y]) continue;
    while (a[y].length <= x) a[y].push(".");
    a[y][x] = ch;
  }
  return a.map((r) => r.join(""));
}

/** Lifts each wing column by its distance from the body, so a front-view glide reads as a V. */
function dihedral(
  g: Grid,
  slopeL: number,
  slopeR: number,
  core = 6,
): Grid {
  const h = g.length,
    w = partW(g),
    cx = Math.floor(w / 2);
  const out = Array.from({ length: h }, () => Array<string>(w).fill("."));
  for (let x = 0; x < w; x++) {
    const d = x - cx,
      far = Math.abs(d) - core;
    const k = far <= 0 ? 0 : Math.round(far * (d < 0 ? slopeL : slopeR));
    for (let y = 0; y < h; y++) {
      const c = g[y][x] ?? ".";
      if (c === ".") continue;
      const ny = y - k;
      if (ny >= 0 && ny < h) out[ny][x] = c;
    }
  }
  return out.map((r) => r.join(""));
}

export function vParts(set: Rig, slope: number, roll: number): void {
  set.parts.glide_v = dihedral(set.parts.glide, slope, slope);
  set.parts.glide_vl = dihedral(set.parts.glide, slope + roll, slope - roll);
  set.parts.glide_vr = dihedral(set.parts.glide, slope - roll, slope + roll);
}

// ---- frames ----
interface PerchOpts {
  dx?: number;
  dy?: number;
  body?: string;
  legs?: string;
  more?: Layer[];
}
/** A perched frame with the head at (x, y), in the body's cells (Zeca's head rests at 14, 1). */
export const perch = (
  head: string,
  x: number,
  y: number,
  ms: number,
  o: PerchOpts = {},
): Frame => ({
  dx: o.dx ?? 0,
  dy: o.dy ?? 0,
  ms,
  layers: [
    [o.body ?? "body", 0, 0],
    [o.legs ?? "legs", 9, 17],
    ["band", 13, 5],
    [head, x, y],
    ...(o.more ?? []),
  ],
});
export const fly = (part: string, ms: number, dy = 0): Frame => ({
  dx: 0,
  dy,
  ms,
  layers: [[part, 0, 0]],
});
/** A flight frame inside a perched clip: shifted so the wings centre over the body. */
export const air = (part: string, ms: number, dy: number, dx = -6): Frame => ({
  dx,
  dy,
  ms,
  layers: [[part, 0, 0]],
});
/** The front-facing sunning pose (wings spread), as in the approval clip. */
export const sun = (
  head: string,
  x: number,
  y: number,
  ms: number,
  dy = 0,
): Frame => ({
  dx: 0,
  dy,
  ms,
  layers: [
    ["sunning", -4, 4],
    ["band", 11, 4],
    [head, x, y],
    ["legs_front", 10, 14],
  ],
});
export const slow = (clip: Clip, k: number): Clip => ({
  ...clip,
  frames: clip.frames.map((f) => ({ ...f, ms: Math.round(f.ms * k) })),
});

/** Flaps, then one long glide: the bigger the bird, the fewer the beats. */
export const soar = (flaps: number, glide: number): Clip => ({
  loop: true,
  frames: [...Array(flaps)]
    .flatMap(() => [
      fly("fly_up", 170),
      fly("glide", 70),
      fly("fly_down", 190, 1),
      fly("glide", 70),
    ])
    .concat([fly("glide", glide)]),
});
/** Cathartes: a beat or two, then a long V-winged glide rocking side to side. */
export function teeterFly(k: number): Clip {
  return {
    loop: true,
    frames: [
      fly("fly_up", 150),
      fly("fly_down", 170, 1),
      fly("glide_v", 420),
      fly("glide_vl", 300 * k),
      fly("glide_v", 180 * k),
      fly("glide_vr", 300 * k),
      fly("glide_v", 260 * k),
      fly("glide_vl", 280 * k),
      fly("glide_v", 180 * k),
      fly("glide_vr", 280 * k),
      fly("glide_v", 500 * k),
    ],
  };
}

// ---- size: grow or shrink the rig along seams ----
// Seams cross the body where it is only fill, spread over a few rows so the feather streaks stay
// diagonal. Every layer past a seam moves with it.
function spread(n: number, k: number): number[] {
  const out = Array<number>(k).fill(Math.trunc(n / k));
  for (let i = 0; i < Math.abs(n % k); i++) out[i] += Math.sign(n);
  return out;
}
function grow<T>(
  a: T[],
  seams: number[],
  counts: number[],
  copyOf: (t: T) => T,
): T[] {
  const out = [...a];
  seams
    .map((s, i) => [s, counts[i]])
    .sort((p, q) => q[0] - p[0])
    .forEach(([s, n]) => {
      // The copies are plain fill: a copied streak would draw a stripe across the new space.
      const copy = copyOf(out[s]);
      if (n > 0) out.splice(s, 0, ...Array<T>(n).fill(copy));
      else if (n < 0) out.splice(s, -n);
    });
  return out;
}
const growRows = (g: Grid, seams: number[], counts: number[]): Grid =>
  grow(g, seams, counts, (r) => r.replace(/[sd]/g, "b"));
/** Grows (or shrinks) each [from, to) column range by n columns spread evenly across it, so a
 *  wing keeps its shape (wrist, curve, fingers) at any span; one copied column drew a flat bar. */
function stretchCols(g: Grid, ranges: [from: number, to: number, n: number][]): Grid {
  const w = partW(g);
  const at = new Map<number, number>();
  for (const [from, to, n] of ranges) {
    const len = to - from;
    for (let i = 0; i < Math.abs(n); i++) {
      const x = from + Math.floor(((i + 0.5) * len) / Math.abs(n));
      at.set(x, (at.get(x) ?? 0) + Math.sign(n));
    }
  }
  return g.map((r) => {
    const cells = [...r.padEnd(w, ".")];
    const out: string[] = [];
    cells.forEach((c, x) => {
      const k = at.get(x) ?? 0;
      if (k < 0) return;
      out.push(c);
      // Copies are plain fill: a copied streak would draw a stripe.
      for (let i = 0; i < k; i++) out.push(c === "s" || c === "d" ? "b" : c);
    });
    return out.join("");
  });
}
function growCols(g: Grid, seams: number[], counts: number[]): Grid {
  const w = partW(g);
  return g.map((r) =>
    grow([...r.padEnd(w, ".")], seams, counts, (c) =>
      c === "s" || c === "d" ? "b" : c,
    ).join(""),
  );
}
const shiftBy = (v: number, seams: number[], counts: number[]) =>
  seams.reduce((acc, s, i) => acc + (v > s ? counts[i] : 0), 0);
const ROWS = [7, 10, 13],
  COLS = [5, 8];

export interface Size {
  rows?: number;
  cols?: number;
  span?: number;
  sun?: number;
}

export function resize(
  set: Rig,
  { rows = 0, cols = 0, span = 0, sun = 0 }: Size = {},
): void {
  const rc = spread(rows, 3),
    cc = spread(cols, 2);
  for (const n of BODIES)
    set.parts[n] = growCols(growRows(set.parts[n], ROWS, rc), COLS, cc);
  const wr = Math.round(rows / 2),
    wc = Math.round(cols / 2);
  set.parts.wing_up = growCols(
    growRows(set.parts.wing_up, [6], [wr]),
    [6],
    [wc],
  );
  // The extra span spreads over the arm and hand, the finger tips left as drawn.
  for (const n of FLIGHT) {
    const w = partW(set.parts[n]);
    set.parts[n] = stretchCols(set.parts[n], [
      [2, 15, span],
      [w - 15, w - 2, span],
    ]);
  }
  for (const n of SUNNING) {
    const sw = partW(set.parts[n]);
    set.parts[n] = stretchCols(set.parts[n], [
      [1, 12, sun],
      [sw - 12, sw - 1, sun],
    ]);
  }
  for (const clip of Object.values(set.clips))
    for (const f of clip.frames) {
      const names = f.layers.map((l) => l[0]);
      if (names.some((p) => p.startsWith("sunning")))
        f.layers = f.layers.map(([p, x, y]): Layer => [
          p,
          p.startsWith("sunning") ? x : x + sun,
          y,
        ]);
      else if (names.some((p) => p.startsWith("body"))) {
        f.layers = f.layers.map(([p, x, y]): Layer =>
          p.startsWith("body")
            ? [p, x, y]
            : p === "wing_up"
              ? [p, x, y - wr]
              : [p, x + shiftBy(x, COLS, cc), y + shiftBy(y, ROWS, rc)],
        );
      }
    }
  set.perch = 20 + rows + (set.tall ?? 0);
}

/** Real measurements to rig deltas: the wingspan follows the real average (37 cells = 150 cm); big
 *  vultures are tall and slim, so length goes mostly into height. */
export function sizeFrom(len: [number, number], span: [number, number]): Size {
  const L = (len[0] + len[1]) / 2,
    S = (span[0] + span[1]) / 2;
  const sp = Math.round((Math.round((37 * S) / 150) - 37) / 2);
  return {
    rows: Math.round((L - 65) * 0.14),
    cols: Math.round((L - 65) * 0.04),
    span: sp,
    sun: Math.round(sp * 0.6),
  };
}

// ---- heads ----
const heads = (set: Rig) =>
  Object.keys(set.parts).filter((n) => n.startsWith("head"));

/** Cathartes: a smaller bare head (the crown row drops), a longer tail, silver along the trailing
 *  edge of the wing. The greater yellow-headed keeps its inner primaries dark. */
export function cathartes(
  set: Rig,
  { tail = 1, darkInner = false } = {},
): void {
  for (const n of heads(set)) {
    const g = [...set.parts[n]],
      i = g.findIndex((r) => /[^.]/.test(r));
    if (i < 0 || !/^[.h]+$/.test(g[i])) continue;
    g[i] = ".".repeat(g[i].length);
    if (g[i + 1]) g[i + 1] = g[i + 1].replace(/H/g, "h");
    set.parts[n] = g;
  }
  const tails = tail >= 2 ? [".KBK", "KK"] : tail === 1 ? [".KK"] : [];
  for (const n of BODIES) set.parts[n] = [...set.parts[n], ...tails];
  for (const n of FLIGHT) {
    const g = set.parts[n],
      cx = Math.floor(partW(g) / 2);
    set.parts[n] = g.map((r) =>
      [...r]
        .map((c, x) =>
          c === "s" && (!darkInner || Math.abs(x - cx) > 9) ? "v" : c,
        )
        .join(""),
    );
  }
}

/** The pixel above each eye becomes forehead ('f'), blinking or not. */
export function forehead(set: Rig): void {
  for (const n of heads(set)) {
    if (n.includes(":")) continue;
    const g = set.parts[n],
      pts: [number, number][] = [];
    g.forEach((r, y) =>
      [...r].forEach((c, x) => {
        if ((c === "E" || c === "e") && y > 0 && "hH".includes(g[y - 1][x]))
          pts.push([x, y - 1]);
      }),
    );
    set.parts[n] = patch(g, pts, "f");
    if (set.parts[n + ":blink"])
      set.parts[n + ":blink"] = patch(set.parts[n + ":blink"], pts, "f");
  }
}

/** King vulture: the orange caruncle ('o') on top of the bill, where the bill starts. */
export function caruncle(name: string, g: Grid): Grid {
  if (name.startsWith("head_up"))
    return patch(
      g,
      [
        [6, 1],
        [6, 2],
      ],
      "o",
    );
  if (name.startsWith("head_front"))
    return patch(
      g,
      [
        [3, 3],
        [4, 3],
      ],
      "o",
    );
  const by = g.findIndex((r) => r.includes("P"));
  if (by < 0) return g;
  const bx = g[by].indexOf("P");
  return patch(
    g,
    [
      [bx, by],
      [bx, by - 1],
    ],
    "o",
  );
}

/** Condor: a fleshy comb ('c') over the crown. Heads gain a row on top, so head layers move up one. */
export function comb(set: Rig): void {
  for (const n of heads(set)) {
    const g = set.parts[n],
      w = partW(g),
      crown = g.findIndex((r) => r.includes("h"));
    const out = [".".repeat(w), ...g.map((r) => r.padEnd(w, "."))];
    const hs = [...g[crown]].flatMap((c, x) => (c === "h" ? [x] : []));
    const mid = Math.floor(hs.length / 2);
    out[crown] = patch(
      [out[crown]],
      hs
        .slice(Math.max(0, mid - 1), mid + 1)
        .map((x): [number, number] => [x, 0]),
      "c",
    )[0];
    set.parts[n] = out;
  }
  for (const clip of Object.values(set.clips))
    for (const f of clip.frames)
      f.layers = f.layers.map(([p, x, y]): Layer => [
        p,
        x,
        p.startsWith("head") ? y - 1 : y,
      ]);
}

/** A longer bill: each row ending in a bill tip grows one pixel forward. */
export function longBill(set: Rig, ch: string): void {
  for (const n of heads(set)) {
    if (n.startsWith("head_back") || n.startsWith("head_front")) continue;
    set.parts[n] = set.parts[n].map((r) => {
      const i = r.search(/[^.][.]*$/);
      return i >= 6 && r[i] === "p"
        ? r.slice(0, i) + ch + "p" + r.slice(i + 2)
        : r;
    });
  }
}

/** Something hanging under the bill base: a beard of bristles, or fleshy lappets. */
export function under(set: Rig, ch: string, n = 2): void {
  for (const name of heads(set)) {
    const g = set.parts[name].map((r) => [...r]);
    let low = -1;
    g.forEach((r, y) => {
      if (r.includes("P")) low = y;
    });
    if (low < 0) continue;
    const xs = g[low].flatMap((c, x) => (c === "P" ? [x] : [])).slice(0, n);
    for (const [k, x] of xs.entries())
      for (let d = 1; d <= (k === 0 ? 2 : 1); d++) {
        while (g.length <= low + d) g.push([]);
        const row = g[low + d];
        while (row.length <= x) row.push(".");
        if (row[x] === ".") row[x] = ch;
      }
    set.parts[name] = g.map((r) => r.join(""));
  }
}

/** Copies of every head with the bare skin flushed red ('Q'): dominance, courtship, agitation. */
export function redHeads(set: Rig, color: string): void {
  set.palette.Q = color;
  for (const n of heads(set)) {
    const [base, blink] = n.split(":");
    set.parts[`${base}R${blink ? ":" + blink : ""}`] = recolor(set.parts[n], {
      H: "Q",
    });
  }
}

export const recolorHeads = (
  set: Rig,
  prefix: string,
  map: Record<string, string>,
): void => {
  for (const n of Object.keys(set.parts))
    if (n.startsWith(prefix)) set.parts[n] = recolor(set.parts[n], map);
};

// ---- bodies ----
/** Where a collar of feathers sits on the body, around the neck socket. */
export const RUFF: [number, number][] = [
  [11, 3],
  [12, 3],
  [12, 4],
  [13, 4],
  [13, 5],
  [12, 5],
  [14, 5],
  [13, 6],
  [14, 6],
  [15, 6],
  [14, 7],
  [15, 7],
];

/** The front of the bird (breast, thighs): the last n fill pixels of each row in [from, to]. */
export function chest(
  set: Rig,
  from: number,
  to: number,
  n: number,
  ch: string,
): void {
  for (const p of BODIES)
    set.parts[p] = set.parts[p].map((r, y) => {
      if (y < from || y > to) return r;
      const a = [...r];
      for (let x = a.length - 1, k = n; x >= 0 && k > 0; x--)
        if (a[x] === "b" || a[x] === "s") {
          a[x] = ch;
          k--;
        }
      return a.join("");
    });
}

export function rect(
  set: Rig,
  parts: string[],
  [x0, x1, y0, y1]: [number, number, number, number],
  map: Record<string, string>,
): void {
  for (const p of parts)
    set.parts[p] = set.parts[p].map((r, y) =>
      y < y0 || y > y1
        ? r
        : [...r].map((c, x) => (x < x0 || x > x1 ? c : (map[c] ?? c))).join(""),
    );
}

/** A scaly mantle: fill pixels on a diagonal lattice become pale feather edges. */
export function scales(set: Rig): void {
  for (const p of BODIES)
    set.parts[p] = set.parts[p].map((r, y) =>
      [...r]
        .map((c, x) => (c === "b" && (x + 2 * y) % 4 === 0 ? "s" : c))
        .join(""),
    );
}

export function tailRows(set: Rig, rows: Grid): void {
  for (const p of BODIES) set.parts[p] = [...set.parts[p], ...rows];
}

/** A pale band along the trailing edge of the spread wings: the lowest `depth` fill cells of each
 *  wing column, the inner one shaded. The body's six middle columns are left as they are. */
export function trailingBand(
  g: Grid,
  depth = 2,
  ch = "W",
  shade = "v",
): Grid {
  const w = partW(g),
    c0 = Math.floor((w - 6) / 2);
  const out = g.map((r) => [...r.padEnd(w, ".")]);
  for (let x = 0; x < w; x++) {
    if (x >= c0 && x < c0 + 6) continue;
    const fill = out.flatMap((r, y) => ("bBsdi".includes(r[x]) ? [y] : []));
    fill.slice(-depth).forEach((y, k) => {
      out[y][x] = k === 0 && depth > 1 ? shade : ch;
    });
  }
  return out.map((r) => r.join(""));
}

/** Recolors the flight frames between `from` and `to` columns away from the body. */
export function flightInner(
  set: Rig,
  from: number,
  to: number,
  map: Record<string, string>,
): void {
  for (const n of FLIGHT) {
    const g = set.parts[n],
      cx = Math.floor(partW(g) / 2);
    set.parts[n] = g.map((r) =>
      [...r]
        .map((c, x) => {
          const d = Math.abs(x - cx);
          return d >= from && d <= to ? (map[c] ?? c) : c;
        })
        .join(""),
    );
  }
}

// A body of its own for the big vultures: tall and vertical, the shoulders (the folded wrists)
// rising behind the neck, a narrow chest, the wing tips and tail hanging low behind the legs. Same
// anchors as Zeca's body, with the legs TALL rows lower.
const TALL_ROWS: [number, number][] = [
  [8, 12],
  [6, 14],
  [5, 15],
  [4, 16],
  [4, 16],
  [3, 16],
  [3, 17],
  [3, 17],
  [3, 17],
  [3, 17],
  [3, 17],
  [3, 17],
  [3, 16],
  [3, 16],
  [3, 16],
  [3, 15],
  [3, 15],
  [3, 14],
  [4, 14],
  [4, 13],
  [3, 12],
  [2, 12],
];
const TALL_LOW: [number, number][][] = [
  [
    [2, 6],
    [8, 13],
  ],
  [
    [1, 5],
    [9, 13],
  ],
  [[1, 4]],
  [[0, 3]],
  [[0, 2]],
];
export const TALL = 7;
/** The wing fold's diagonal (x + y) on the tall body. */
const FOLD = 17;

function tallGrid(puff: boolean): Grid {
  const spans = [
    ...TALL_ROWS.map(([a, b], y): [number, number][] => [
      puff && y > 0 && y < 21 ? [a - 1, b + 1] : [a, b],
    ]),
    ...TALL_LOW,
  ];
  const inside = (x: number, y: number) =>
    y >= 0 && y < spans.length && spans[y].some(([a, b]) => x >= a && x <= b);
  return spans.map((_, y) =>
    Array.from({ length: 24 }, (_, x) => {
      if (!inside(x, y)) return ".";
      if (
        !inside(x - 1, y) ||
        !inside(x + 1, y) ||
        !inside(x, y - 1) ||
        !inside(x, y + 1)
      )
        return "K";
      if (!inside(x - 2, y) || !inside(x + 2, y) || !inside(x, y - 2))
        // Light from above-left: the ring is lit along the back (the shoulders' top and the left
        // side), never toward the neck or the chest.
        return x <= 11 && y < 14 && inside(x + 2, y) ? "i" : "B";
      // The folded wing's trailing edge, a shadowed diagonal from the neck toward the tail; it
      // stays above row 16, where gyps() recolors the flight feathers cell by cell.
      if (x + y === FOLD && y >= 4 && y <= 14 && x < 14) return "d";
      // Feather rows inside the wing panel (above the fold), diagonal like Zeca's; the belly is plain.
      return x < 14 && y > 2 && y < 19 && x + y < FOLD && (x - y + 40) % 6 === 0
        ? "s"
        : "b";
    }).join(""),
  );
}
export function tallBody(set: Rig): void {
  set.parts.body = tallGrid(false);
  set.parts.body_puff = tallGrid(true);
  set.tall = TALL;
}
export function tallLegs(set: Rig): void {
  for (const clip of Object.values(set.clips))
    for (const f of clip.frames)
      if (f.layers.some(([p]) => p.startsWith("body")))
        f.layers = f.layers.map(([p, x, y]): Layer => [
          p,
          x,
          p.startsWith("legs") ? y + TALL : y,
        ]);
}

/**
 * Long necks: the head rises k rows and a neck column fills the gap, in bare skin or down, with a
 * collar where it leaves the body. Lowered heads (feeding, preening) keep their place.
 */
export function neck(
  set: Rig,
  k: number,
  edge: string,
  fill: string,
  ruff?: string,
): void {
  if (ruff)
    set.parts.ruff = [
      `.${ruff}${ruff}${ruff}.`,
      ruff.repeat(5),
      `.${ruff}${ruff}${ruff}.`,
    ];
  const at: Record<string, [number, number] | null> = {};
  for (const n of heads(set)) {
    let pos: [number, number] | null = null;
    set.parts[n].forEach((r, y) => {
      const x = [...r].findIndex((c) => c === "w" || c === "H");
      if (x >= 0) pos = [x, y];
    });
    at[n] = pos;
  }
  // Where the neck meets the body: a raised head always reaches down to it, so a stretch never
  // leaves a gap.
  const rest = set.clips.idle.frames[0].layers.find(([p]) => p === "head")!;
  const J = rest[2] + at.head![1];
  // At rest a long-necked vulture sinks its neck into the ruff; it stretches only when alert.
  for (const [name, clip] of Object.entries(set.clips))
    for (const f of clip.frames) {
      const onBody = f.layers.some(([p]) => p.startsWith("body"));
      if (!onBody && !f.layers.some(([p]) => p.startsWith("sunning"))) continue;
      const kk = ["idle", "sleep", "read", "edit", "run", "preen"].includes(
        name,
      )
        ? Math.min(1, k)
        : k;
      let raised = false;
      f.layers = f.layers.flatMap(([p, x, y]): Layer[] => {
        const pos = at[p];
        if (!p.startsWith("head") || p.startsWith("head_down") || !pos)
          return [[p, x, y]];
        const [nx, ny] = pos,
          top = y - kk + ny,
          bottom = onBody ? Math.max(top + kk, J) : top + kk,
          part = `neck_${bottom - top + 1}`;
        set.parts[part] ??= Array(bottom - top + 1).fill(edge + fill + fill);
        raised = true;
        return [
          [part, x + nx, top],
          ...(ruff ? [["ruff", x + nx - 1, bottom - 1] as Layer] : []),
          [p, x, y - kk],
        ];
      });
      // The agent's band stays on top of the collar.
      if (raised)
        f.layers = [
          ...f.layers.filter(([p]) => p !== "band"),
          ...f.layers.filter(([p]) => p === "band"),
        ];
    }
}
