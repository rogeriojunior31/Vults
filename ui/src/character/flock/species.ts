// The 23 vulture species, each built from Zeca's rig: a palette, a size from the real bird's
// measurements, a few pixel details, and only the clips where the real bird behaves differently.
// Every species also has a signature: one clip of its own, from something the real bird does.
import type { Clip } from "../sprites";
import {
  BODIES,
  FLIGHT,
  RUFF,
  air,
  caruncle,
  cathartes,
  chest,
  comb,
  flightInner,
  fly,
  forehead,
  longBill,
  patch,
  perch,
  recolor,
  recolorHeads,
  rect,
  redHeads,
  scales,
  sizeFrom,
  slow,
  soar,
  sun,
  tailRows,
  teeterFly,
  under,
  type Rig,
  type Size,
} from "./rig";

type Family = "new-world" | "old-world";
type Iucn = "LC" | "NT" | "VU" | "EN" | "CR";

export interface Species {
  id: string;
  /** English name (a translation comes with the i18n catalog). */
  name: string;
  latin: string;
  family: Family;
  iucn: Iucn;
  /** Length and wingspan in cm (ranges from field guides); they set the size. */
  length: [number, number];
  span: [number, number];
  /** Rig deltas, when they are tuned by hand instead of computed from the measurements. */
  size?: Size;
  /** The tall body of the big vultures. */
  tall?: boolean;
  /** Neck rows when alert, its edge and fill colors, and the collar's color. */
  neck?: [number, string, string, string?];
  /** V-winged glide: slope and roll of the teeter. */
  v?: [number, number];
  /** Colors over Zeca's; an empty string is a see-through pixel (Cathartes nostrils). */
  palette: Record<string, string>;
  build?: (set: Rig) => void;
  signature: { label: string; clip: (set: Rig) => Clip };
}

function gyps(set: Rig, ruff = "u"): void {
  for (const p of BODIES)
    set.parts[p] = patch(
      recolor(set.parts[p], { b: "x", s: "x", B: "K" }, set.tall ? 16 : 12),
      RUFF,
      ruff,
    );
  for (const n of FLIGHT)
    set.parts[n] = recolor(set.parts[n], { W: "x", v: "K" });
  set.clips.fly = soar(2, 2800);
  set.clips.idle = slow(set.clips.idle, 1.25);
}

const SCRAP = (set: Rig) => {
  set.parts.scrap = ["MM", "M."];
  set.palette.M = "#a8323a";
};

export const SPECIES: Species[] = [
  {
    id: "atratus",
    name: "Black vulture",
    latin: "Coragyps atratus",
    family: "new-world",
    iucn: "LC",
    length: [56, 74],
    span: [133, 167],
    size: {},
    palette: {},
    signature: {
      // Urohidrosis: it cools off by excreting on its own legs, which is why they look white.
      label: "cools off",
      clip: (s) => {
        s.parts.drip = ["W"];
        s.parts.legs_wet = recolor(s.parts.legs, { L: "W", l: "v" });
        return {
          loop: false,
          frames: [
            perch("head", 14, 1, 500),
            perch("head_down", 11, 7, 400),
            perch("head_down", 11, 7, 120, { more: [["drip", 10, 15]] }),
            perch("head_down", 11, 7, 120, { more: [["drip", 10, 17]] }),
            perch("head_down", 11, 7, 300, { legs: "legs_wet" }),
            perch("head_down:blink", 11, 7, 140, { legs: "legs_wet" }),
            perch("head", 14, 1, 900, { legs: "legs_wet" }),
          ],
        };
      },
    },
  },
  {
    id: "aura",
    name: "Turkey vulture",
    latin: "Cathartes aura",
    family: "new-world",
    iucn: "LC",
    length: [62, 81],
    span: [160, 183],
    size: { rows: 1, cols: 1, span: 3, sun: 2 },
    v: [0.42, 0.2],
    palette: {
      K: "#140e0c",
      B: "#2a1d18",
      b: "#3b2b23",
      s: "#6e5241",
      H: "#bb3a2d",
      h: "#e0705a",
      w: "#7e231c",
      e: "#f0e6d6",
      N: "",
      P: "#e9dfcc",
      p: "#fbf6ea",
      L: "#e2b4a6",
      l: "#b3847a",
      W: "#d9d4cc",
      v: "#a9a39a",
    },
    build(set) {
      cathartes(set, { tail: 2 });
      set.clips.fly = teeterFly(1);
      // It finds food by smell: head up, sniffing the air.
      set.clips.search = {
        loop: true,
        frames: [
          perch("head_up", 14, 0, 380),
          perch("head_up", 14, -1, 120),
          perch("head_up", 14, 0, 120),
          perch("head_up", 14, -1, 120),
          perch("head_up", 14, 0, 300),
          perch("head", 15, 1, 360),
          perch("head_up:blink", 14, 0, 120),
          perch("head_up", 14, 0, 260),
        ],
      };
    },
    signature: {
      // The horaltic pose: wings spread to the morning sun.
      label: "sunning",
      clip: () => ({
        loop: false,
        frames: [
          sun("head", 10, -1, 700),
          sun("head", 10, -2, 500),
          sun("head:blink", 10, -2, 120),
          sun("head_back", 5, -1, 700),
          sun("head", 10, -1, 900),
        ],
      }),
    },
  },
  {
    id: "burrovianus",
    name: "Lesser yellow-headed vulture",
    latin: "Cathartes burrovianus",
    family: "new-world",
    iucn: "LC",
    length: [53, 66],
    span: [150, 165],
    size: { rows: -1, cols: -1, span: 1, sun: 1 },
    v: [0.36, 0.26],
    palette: {
      K: "#0c0d0b",
      B: "#1d1e1a",
      b: "#2a2b25",
      s: "#474b40",
      H: "#f2b233",
      h: "#7391b8",
      w: "#d4542e",
      f: "#d8352a",
      e: "#c8342a",
      N: "",
      P: "#e6cfb4",
      p: "#faf2e4",
      L: "#e8e3da",
      l: "#b6aea3",
      W: "#efeee9",
      v: "#c4c2b9",
    },
    build(set) {
      cathartes(set, { tail: 0 });
      forehead(set);
      set.clips.fly = teeterFly(0.62);
      // Head low, sweeping the grass.
      set.clips.search = {
        loop: true,
        frames: [
          perch("head_down", 16, 5, 260, { dy: 1 }),
          perch("head_down", 17, 5, 220, { dy: 1 }),
          perch("head_down", 18, 6, 240, { dy: 1 }),
          perch("head_down", 17, 6, 200, { dy: 1 }),
          perch("head_down:blink", 16, 5, 120, { dy: 1 }),
          perch("head_down", 15, 4, 260),
          perch("head", 14, 1, 420),
          perch("head_back", 9, 0, 380),
        ],
      };
    },
    signature: {
      label: "low over the grass",
      clip: () => ({
        loop: true,
        frames: [
          air("glide_vl", 180, 2, 0),
          air("glide_v", 120, 3, 0),
          air("glide_vr", 180, 2, 0),
          air("glide_v", 120, 1, 0),
          air("glide_vl", 180, 2, 0),
          air("glide_v", 120, 3, 0),
          air("glide_vr", 180, 2, 0),
          air("glide_v", 300, 1, 0),
        ],
      }),
    },
  },
  {
    id: "melambrotus",
    name: "Greater yellow-headed vulture",
    latin: "Cathartes melambrotus",
    family: "new-world",
    iucn: "LC",
    length: [64, 75],
    span: [166, 178],
    size: { rows: 1, cols: 2, span: 3, sun: 2 },
    v: [0.3, 0.12],
    palette: {
      K: "#050607",
      B: "#0e1013",
      b: "#171a20",
      s: "#2f3644",
      H: "#f6c62c",
      h: "#5a6f9a",
      w: "#e0a090",
      e: "#c8342a",
      N: "",
      P: "#e0c8a6",
      p: "#f8efe0",
      L: "#2a2a2c",
      l: "#161618",
      W: "#e6e5df",
      v: "#b4b2aa",
    },
    build(set) {
      cathartes(set, { tail: 2, darkInner: true });
      set.clips.fly = teeterFly(1.35);
      // Peering down between the treetops.
      set.clips.search = {
        loop: true,
        frames: [
          perch("head_tilt", 15, 1, 520),
          perch("head_tilt", 16, 2, 360),
          perch("head_tilt:blink", 16, 2, 120),
          perch("head_down", 16, 4, 420),
          perch("head_tilt", 15, 1, 400),
          perch("head", 14, 1, 300),
          perch("head_back", 9, 0, 520),
        ],
      };
    },
    signature: {
      // It finds the carcass but needs the king vulture to open it.
      label: "waits for the king",
      clip: () => ({
        loop: false,
        frames: [
          perch("head_up", 14, 0, 600),
          perch("head_back", 9, 0, 500),
          perch("head_up", 14, 0, 300),
          perch("head_up:blink", 14, 0, 120),
          perch("head_tilt", 15, 0, 600),
          perch("head_up", 14, -1, 500),
          perch("head", 14, 1, 600),
        ],
      }),
    },
  },
  {
    id: "papa",
    name: "King vulture",
    latin: "Sarcoramphus papa",
    family: "new-world",
    iucn: "LC",
    length: [67, 81],
    span: [120, 200],
    size: { rows: 2, cols: 2, span: 2, sun: 2 },
    palette: {
      K: "#0b0a0c",
      B: "#141316",
      b: "#f1e4d0",
      s: "#cdc2ab",
      H: "#f0952e",
      h: "#d9452f",
      w: "#6f4b91",
      E: "#f6f1e6",
      e: "#d2302f",
      N: "#f2c94c",
      P: "#1d1a19",
      p: "#e8582f",
      L: "#8e8a92",
      l: "#6b6770",
      W: "#2b292d",
      v: "#141316",
      o: "#f7a12b",
      g: "#4a4850",
    },
    build(set) {
      for (const n of BODIES)
        set.parts[n] = patch(
          recolor(set.parts[n], { b: "B", s: "K" }, 11),
          RUFF,
          "g",
        );
      for (const n of Object.keys(set.parts))
        if (n.startsWith("head") || FLIGHT.includes(n))
          set.parts[n] = caruncle(n, set.parts[n]);
      for (const n of FLIGHT) set.parts[n] = recolor(set.parts[n], { s: "B" });
      set.clips.fly = {
        loop: true,
        frames: [
          fly("fly_up", 150),
          fly("glide", 100),
          fly("fly_down", 170, 1),
          fly("glide", 100),
          fly("fly_up", 150),
          fly("glide", 100),
          fly("fly_down", 170, 1),
          fly("glide", 2400),
        ],
      };
      set.clips.idle = slow(set.clips.idle, 1.3);
      set.clips.think = slow(set.clips.think, 1.3);
      const up = (dy: number) =>
        perch("head_up", 14, 0, 520, { more: [["wing_up", 0, dy]] });
      // A double stretch: the king takes its time.
      set.clips.done = {
        loop: false,
        frames: [
          perch("head", 14, 1, 160, { dy: 1 }),
          perch("head", 14, 1, 140, { dy: -1 }),
          perch("head", 14, 0, 140, { more: [["wing_up", 1, -7]] }),
          up(-9),
          perch("head", 14, 1, 160, { more: [["wing_up", 1, -6]] }),
          perch("head", 14, 1, 260),
          perch("head", 14, 0, 140, { more: [["wing_up", 1, -7]] }),
          up(-10),
          perch("head", 14, 1, 900),
        ],
      };
    },
    signature: {
      // The strongest bill of the New World vultures tears the hide the others cannot.
      label: "opens the carcass",
      clip: (s) => {
        SCRAP(s);
        return {
          loop: false,
          frames: [
            perch("head", 14, 1, 300),
            perch("head_down", 16, 6, 90, { dx: 1, dy: 1 }),
            perch("head_down", 16, 7, 160, { dx: 1, dy: 1 }),
            perch("head_down", 14, 4, 120, {
              dx: -1,
              more: [["scrap", 20, 11]],
            }),
            perch("head_up", 14, 0, 260, { more: [["scrap", 22, -2]] }),
            perch("head_up:blink", 14, -1, 160),
            perch("head_down", 16, 6, 90, { dx: 1, dy: 1 }),
            perch("head_down", 14, 4, 140, {
              dx: -1,
              more: [["scrap", 20, 11]],
            }),
            perch("head", 14, 1, 800),
          ],
        };
      },
    },
  },
  {
    id: "vultur",
    name: "Andean condor",
    latin: "Vultur gryphus",
    family: "new-world",
    iucn: "VU",
    length: [100, 130],
    span: [270, 320],
    size: { rows: 7, cols: 2, span: 18, sun: 10 },
    tall: true,
    neck: [3, "w", "H", "u"],
    palette: {
      K: "#070709",
      B: "#111114",
      b: "#1b1b20",
      s: "#34343b",
      W: "#f2f0ea",
      v: "#c9c7c0",
      H: "#7a3a36",
      h: "#5c2a2a",
      c: "#3e1a1c",
      w: "#4a2424",
      E: "#1a0f0d",
      e: "#c99a6a",
      N: "#3d2422",
      P: "#2a2523",
      p: "#ddd3c2",
      L: "#8c8a90",
      l: "#67656b",
      u: "#f5f3ee",
    },
    build(set) {
      for (const n of BODIES)
        set.parts[n] = patch(
          recolor(set.parts[n], { s: "W" }, 4, 11),
          RUFF,
          "u",
        );
      set.parts.sunning = recolor(set.parts.sunning, { s: "W" });
      comb(set);
      set.clips.fly = {
        loop: true,
        frames: [
          fly("fly_up", 200),
          fly("fly_down", 230, 1),
          fly("glide", 3600),
        ],
      };
      set.clips.idle = slow(set.clips.idle, 1.5);
    },
    signature: {
      // Its head flushes red within seconds: a dominance signal.
      label: "red head",
      clip: (s) => {
        redHeads(s, "#ec3a2c");
        return {
          loop: false,
          frames: [
            perch("head", 14, 1, 500),
            perch("head_up", 14, 0, 300),
            perch("head_upR", 14, 0, 200),
            perch("head_upR", 14, -1, 600, { body: "body_puff" }),
            perch("headR", 14, 1, 500, { body: "body_puff" }),
            perch("headR:blink", 14, 1, 120, { body: "body_puff" }),
            perch("headR", 14, 1, 600, { body: "body_puff" }),
            perch("head", 14, 1, 800),
          ],
        };
      },
    },
  },
  {
    id: "gymnogyps",
    name: "California condor",
    latin: "Gymnogyps californianus",
    family: "new-world",
    iucn: "CR",
    length: [109, 140],
    span: [250, 300],
    tall: true,
    neck: [3, "w", "H", "B"],
    palette: {
      K: "#08080a",
      B: "#121215",
      b: "#1d1d22",
      s: "#3a3a42",
      H: "#e8894a",
      h: "#efa27a",
      w: "#c65a3c",
      E: "#2a0a0a",
      e: "#a8402e",
      N: "#8a3a2a",
      P: "#e9d6b0",
      p: "#f5ead2",
      L: "#9a9aa0",
      l: "#727278",
      T: "#e94b3c",
      t: "#f5f5f5",
    },
    build(set) {
      for (const p of BODIES)
        set.parts[p] = patch(
          recolor(set.parts[p], { s: "v" }, 5, 7),
          RUFF,
          "K",
        );
      // Every wild California condor carries a numbered wing tag.
      for (const p of BODIES)
        set.parts[p] = patch(
          patch(
            set.parts[p],
            [
              [6, 9],
              [7, 9],
              [6, 10],
            ],
            "T",
          ),
          [[7, 10]],
          "t",
        );
      flightInner(set, 4, 15, { s: "W", b: "W" });
      set.clips.fly = soar(1, 3600);
      set.clips.idle = slow(set.clips.idle, 1.5);
    },
    signature: {
      // Courtship: head flushed red, wings spread, a bow.
      label: "courtship",
      clip: (s) => {
        redHeads(s, "#d8302a");
        return {
          loop: false,
          frames: [
            perch("head", 14, 1, 400),
            sun("head_frontR", 8, -2, 500),
            sun("head_frontR", 8, -1, 300, 1),
            sun("head_frontR", 8, 0, 300, 2),
            sun("head_frontR:blink", 8, -1, 140, 1),
            sun("head_frontR", 8, -2, 600),
            perch("head", 14, 1, 600),
          ],
        };
      },
    },
  },
  {
    id: "neophron",
    name: "Egyptian vulture",
    latin: "Neophron percnopterus",
    family: "old-world",
    iucn: "EN",
    length: [47, 66],
    span: [146, 175],
    neck: [1, "h", "h"],
    palette: {
      K: "#121110",
      B: "#1e1c1a",
      b: "#efe8da",
      s: "#d3c9b3",
      H: "#f2c230",
      h: "#f6f1e6",
      w: "#e3a52a",
      E: "#3a1a0a",
      e: "#e8d090",
      N: "#1e1a18",
      P: "#e0a020",
      p: "#1e1a18",
      L: "#e8b0a0",
      l: "#c08878",
      m: "#8a857c",
    },
    build(set) {
      for (const p of BODIES)
        set.parts[p] = recolor(
          recolor(set.parts[p], { b: "B", s: "K" }, 11),
          { B: "b" },
          16,
        );
      tailRows(set, [".KbK", "KbK", "KK"]);
      set.parts.stone = ["mm", "mm"];
      set.clips.run = {
        loop: true,
        frames: [
          perch("head_up", 14, 0, 300, { more: [["stone", 21, -2]] }),
          perch("head_up", 14, -1, 160, { dy: -1, more: [["stone", 21, -3]] }),
          perch("head_down", 15, 5, 70, { dy: 1, more: [["stone", 20, 13]] }),
          perch("head_down", 15, 6, 180, { dy: 1, more: [["stone", 20, 14]] }),
          perch("head_down", 15, 4, 200, { more: [["stone", 20, 12]] }),
        ],
      };
      set.clips.fly = soar(3, 1600);
    },
    signature: {
      // A tool user: it throws a stone at an ostrich egg until the shell breaks.
      label: "throws a stone",
      clip: () => ({
        loop: false,
        frames: [
          perch("head_down", 15, 5, 300, { more: [["stone", 20, 14]] }),
          perch("head_up", 14, 0, 260, { more: [["stone", 21, -2]] }),
          perch("head_up", 14, -1, 200, { dy: -1, more: [["stone", 21, -3]] }),
          perch("head_down", 15, 5, 60, { dy: 1, more: [["stone", 23, 10]] }),
          perch("head_down", 15, 6, 80, { dy: 1, more: [["stone", 24, 14]] }),
          perch("head_down", 15, 5, 400, { more: [["stone", 24, 15]] }),
          perch("head", 14, 1, 700),
        ],
      }),
    },
  },
  {
    id: "gypaetus",
    name: "Bearded vulture",
    latin: "Gypaetus barbatus",
    family: "old-world",
    iucn: "NT",
    length: [94, 125],
    span: [231, 283],
    tall: true,
    neck: [2, "q", "q"],
    palette: {
      K: "#121316",
      B: "#24262b",
      b: "#3b3d42",
      s: "#9aa0a8",
      H: "#efe2c4",
      h: "#f6ecd6",
      w: "#16130f",
      E: "#d43a2e",
      e: "#f3e7c0",
      N: "#16130f",
      P: "#5b5a5c",
      p: "#3a393b",
      L: "#d08a3a",
      l: "#8a8a90",
      q: "#d9822b",
      k: "#16130f",
    },
    build(set) {
      // Its orange comes from bathing in iron-rich mud.
      chest(set, 3, set.tall ? 18 : 12, set.tall ? 5 : 4, "q");
      under(set, "k", 2);
      tailRows(set, [".KBK", "KBK", "KK"]);
      set.clips.swallow = slow(set.clips.swallow, 1.4);
      set.clips.fly = soar(1, 3000);
    },
    signature: {
      // It eats bone: carries it up and drops it on the rocks to break it.
      label: "drops a bone",
      clip: (s) => {
        s.parts.bone = ["O.O", ".O.", ".O.", "O.O"];
        s.palette.O = "#efe8d6";
        return {
          loop: false,
          frames: [
            perch("head_up", 14, 0, 500, { more: [["bone", 21, -4]] }),
            perch("head_up", 14, -1, 300, { more: [["bone", 21, -5]] }),
            perch("head_up", 14, 0, 90, { more: [["bone", 21, 1]] }),
            perch("head_down", 15, 3, 90, { more: [["bone", 21, 6]] }),
            perch("head_down", 15, 4, 90, { more: [["bone", 21, 10]] }),
            perch("head_down", 15, 5, 120, { more: [["bone", 21, 13]] }),
            perch("head_down", 15, 5, 500),
            perch("head", 14, 1, 600),
          ],
        };
      },
    },
  },
  {
    id: "gypohierax",
    name: "Palm-nut vulture",
    latin: "Gypohierax angolensis",
    family: "old-world",
    iucn: "LC",
    length: [57, 65],
    span: [135, 155],
    neck: [1, "H", "H"],
    palette: {
      K: "#121212",
      B: "#1c1c1e",
      b: "#f2f0ea",
      s: "#d6d3cb",
      H: "#f4f2ec",
      h: "#ffffff",
      w: "#e2552f",
      f: "#e2552f",
      E: "#3a2210",
      e: "#e8c870",
      N: "#8d8577",
      P: "#c8b58a",
      p: "#8d8577",
      L: "#d8a0a0",
      l: "#b07878",
      n: "#e8762a",
    },
    build(set) {
      rect(set, BODIES, [0, 9, 4, 30], { b: "B", s: "K" });
      for (const p of BODIES)
        set.parts[p] = recolor(set.parts[p], { b: "B", s: "K" }, 14);
      forehead(set);
      // Mostly a fruit eater: it picks an oil-palm nut before swallowing.
      set.parts.nut = ["nn", "nn"];
      const sw = set.clips.swallow.frames;
      sw[0].layers.push(["nut", 21, 12]);
      sw[1].layers.push(["nut", 21, 14]);
      sw[2].layers.push(["nut", 20, 11]);
      set.clips.fly = soar(4, 700);
    },
    signature: {
      label: "walks the beach",
      clip: () => ({
        loop: false,
        frames: [
          perch("head", 14, 1, 220, { legs: "legs_step" }),
          perch("head", 15, 2, 220, { dx: 1 }),
          perch("head", 14, 1, 220, { dx: 2, legs: "legs_step" }),
          perch("head", 15, 2, 220, { dx: 3 }),
          perch("head_down", 15, 5, 400, { dx: 3 }),
          perch("head", 14, 1, 220, { dx: 2, legs: "legs_step" }),
          perch("head", 15, 2, 220, { dx: 1 }),
          perch("head", 14, 1, 600),
        ],
      }),
    },
  },
  {
    id: "necrosyrtes",
    name: "Hooded vulture",
    latin: "Necrosyrtes monachus",
    family: "old-world",
    iucn: "CR",
    length: [62, 72],
    span: [155, 180],
    neck: [1, "h", "h"],
    palette: {
      K: "#1a130f",
      B: "#33271f",
      b: "#4a3a2e",
      s: "#6e5a48",
      H: "#f0c4c4",
      h: "#c9b8a6",
      w: "#b3a08c",
      f: "#d83a3a",
      E: "#2a1410",
      e: "#e8d0b0",
      N: "#6a5a4e",
      P: "#6a5a4e",
      p: "#3a302a",
      L: "#9aa0b0",
      l: "#6e7484",
    },
    build(set) {
      // The face flushes red when it is agitated: hissing, or asking for a human.
      recolorHeads(set, "head_hiss", { H: "f" });
      recolorHeads(set, "head_front", { H: "f" });
      set.clips.fly = soar(3, 1400);
    },
    signature: {
      label: "flushes red",
      clip: (s) => {
        redHeads(s, "#d83a3a");
        return {
          loop: false,
          frames: [
            perch("head", 14, 1, 500),
            perch("head_tilt", 15, 1, 300),
            perch("headR", 14, 1, 160),
            perch("headR", 15, 1, 120),
            perch("headR", 14, 1, 120),
            perch("headR", 15, 1, 120),
            perch("headR:blink", 14, 1, 120),
            perch("headR", 14, 1, 500),
            perch("head", 14, 1, 600),
          ],
        };
      },
    },
  },
  {
    id: "gyps-fulvus",
    name: "Griffon vulture",
    latin: "Gyps fulvus",
    family: "old-world",
    iucn: "LC",
    length: [93, 122],
    span: [230, 280],
    tall: true,
    neck: [3, "w", "H", "u"],
    palette: {
      K: "#3a2a18",
      B: "#8a6a40",
      b: "#b8935e",
      s: "#d3b583",
      x: "#2e241c",
      H: "#e8e2d6",
      h: "#f4f0e8",
      w: "#c8bfb0",
      E: "#3a2410",
      e: "#d9b060",
      N: "#7a6a50",
      P: "#d9cbb0",
      p: "#c8b896",
      L: "#9a9898",
      l: "#727070",
      u: "#f2ede2",
    },
    build: (set) => gyps(set),
    signature: {
      // At rest the neck hides in the ruff; alert, it stretches and looks around.
      label: "stretches its neck",
      clip: () => ({
        loop: false,
        frames: [
          perch("head", 14, 1, 400),
          perch("head", 14, 0, 160),
          perch("head", 14, -1, 160),
          perch("head", 14, -2, 500),
          perch("head_back", 9, -3, 500),
          perch("head:blink", 14, -2, 120),
          perch("head_tilt", 15, -2, 500),
          perch("head", 14, -1, 200),
          perch("head", 14, 1, 600),
        ],
      }),
    },
  },
  {
    id: "gyps-rueppelli",
    name: "Rüppell's vulture",
    latin: "Gyps rueppelli",
    family: "old-world",
    iucn: "CR",
    length: [85, 103],
    span: [226, 260],
    tall: true,
    neck: [3, "w", "H", "u"],
    palette: {
      K: "#1a140e",
      B: "#2e241a",
      b: "#4a3b2c",
      s: "#e6d6b4",
      x: "#1f1912",
      H: "#c9c3bb",
      h: "#dcd7cf",
      w: "#a59e95",
      E: "#2a1a08",
      e: "#e0b040",
      N: "#8a7a50",
      P: "#e2cf9a",
      p: "#efe3c0",
      L: "#6a6a70",
      l: "#4a4a50",
      u: "#f2ede2",
    },
    build(set) {
      scales(set);
      gyps(set);
      set.clips.fly = soar(2, 3600);
    },
    signature: {
      // The highest bird flight on record: 11,300 m, in 1973.
      label: "climbs higher",
      clip: () => ({
        loop: false,
        frames: [
          air("glide", 300, 6),
          air("glide", 300, 4),
          air("glide", 300, 2),
          air("glide", 300, 0),
          air("glide", 300, -2),
          air("glide", 300, -4),
          air("glide", 700, -6),
        ],
      }),
    },
  },
  {
    id: "gyps-coprotheres",
    name: "Cape vulture",
    latin: "Gyps coprotheres",
    family: "old-world",
    iucn: "VU",
    length: [96, 115],
    span: [226, 260],
    tall: true,
    neck: [3, "w", "H", "u"],
    palette: {
      K: "#3a2e1e",
      B: "#a88f62",
      b: "#e6d6b0",
      s: "#c9b688",
      x: "#2a2218",
      H: "#d8dbe6",
      h: "#eef0f4",
      w: "#7a9ac8",
      E: "#f0d878",
      e: "#1a1a1a",
      N: "#3a4a6a",
      P: "#3a3a40",
      p: "#6a6a70",
      L: "#8a8a90",
      l: "#6a6a70",
      u: "#efe6d0",
    },
    build: (set) => gyps(set),
    signature: {
      // Two bare blue patches at the base of the neck sense the temperature.
      label: "feels the heat",
      clip: (s) => {
        s.parts.patches = ["w.w"];
        return {
          loop: false,
          frames: [
            perch("head", 14, 1, 400),
            perch("head_up", 14, -1, 600, { more: [["patches", 15, 6]] }),
            perch("head_up:blink", 14, -1, 160, { more: [["patches", 15, 6]] }),
            perch("head_back", 9, -1, 600, { more: [["patches", 15, 6]] }),
            perch("head_up", 14, -2, 600, { more: [["patches", 15, 6]] }),
            perch("head", 14, 1, 600),
          ],
        };
      },
    },
  },
  {
    id: "gyps-himalayensis",
    name: "Himalayan vulture",
    latin: "Gyps himalayensis",
    family: "old-world",
    iucn: "NT",
    length: [103, 115],
    span: [256, 310],
    tall: true,
    neck: [3, "w", "H", "u"],
    palette: {
      K: "#4a3a24",
      B: "#bfae88",
      b: "#e9dfc6",
      s: "#cbbd9c",
      x: "#2e2418",
      H: "#efe6c8",
      h: "#faf8f2",
      w: "#a8c0e0",
      E: "#2a1a10",
      e: "#c8a060",
      N: "#8a7a60",
      P: "#e2d6b8",
      p: "#cfc2a2",
      L: "#a8a890",
      l: "#727070",
      u: "#cdb58e",
    },
    build(set) {
      gyps(set);
      set.clips.fly = soar(1, 3400);
    },
    signature: {
      // It lives above 4,000 m: after a snowfall it fluffs up and shakes the snow off.
      label: "shakes off the snow",
      clip: (s) => {
        s.parts.snow = ["Z..Z.Z", ".Z.Z..", "Z...Z."];
        s.parts.flake = ["Z.Z", "...", ".Z."];
        s.palette.Z = "#f4f6fa";
        return {
          loop: false,
          frames: [
            perch("head", 14, 1, 700, { more: [["snow", 4, 0]] }),
            perch("head", 14, 1, 120, {
              body: "body_puff",
              more: [["snow", 4, 0]],
            }),
            perch("head", 13, 1, 100, {
              body: "body_puff",
              dx: -1,
              more: [
                ["flake", 1, -2],
                ["flake", 10, -3],
              ],
            }),
            perch("head", 15, 1, 100, {
              body: "body_puff",
              dx: 1,
              more: [
                ["flake", 0, -4],
                ["flake", 12, -5],
              ],
            }),
            perch("head", 13, 1, 100, {
              body: "body_puff",
              dx: -1,
              more: [["flake", -1, -6]],
            }),
            perch("head:blink", 14, 1, 160),
            perch("head", 14, 1, 700),
          ],
        };
      },
    },
  },
  {
    id: "gyps-africanus",
    name: "White-backed vulture",
    latin: "Gyps africanus",
    family: "old-world",
    iucn: "CR",
    length: [78, 98],
    span: [196, 225],
    tall: true,
    neck: [3, "w", "H", "u"],
    palette: {
      K: "#1e1610",
      B: "#4a3a2a",
      b: "#6a553c",
      s: "#9a8262",
      x: "#1a140e",
      H: "#6a6866",
      h: "#8a8886",
      w: "#3a3a3a",
      E: "#1a1410",
      e: "#6a5030",
      N: "#2a2a2a",
      P: "#2a2a2c",
      p: "#4a4a4e",
      L: "#2a2a2c",
      l: "#1a1a1c",
      u: "#f2ede2",
    },
    build(set) {
      rect(set, BODIES, [2, 5, 10, 12], { b: "u", s: "u" });
      gyps(set);
    },
    signature: {
      // It drops onto a carcass in a crowd, braking with its wings.
      label: "lands in a crowd",
      clip: () => ({
        loop: false,
        frames: [
          air("glide", 260, -10),
          air("glide", 200, -6),
          air("fly_up", 160, -3),
          perch("head_up", 14, 0, 160, { more: [["wing_up", 0, -9]] }),
          perch("head", 14, 1, 160, { dy: 1, more: [["wing_up", 1, -6]] }),
          perch("head", 14, 1, 800),
        ],
      }),
    },
  },
  {
    id: "gyps-indicus",
    name: "Indian vulture",
    latin: "Gyps indicus",
    family: "old-world",
    iucn: "CR",
    length: [80, 100],
    span: [205, 229],
    tall: true,
    neck: [3, "w", "H", "u"],
    palette: {
      K: "#3a2e1e",
      B: "#a88f62",
      b: "#d8c49a",
      s: "#bfa77a",
      x: "#2e241a",
      H: "#5a5048",
      h: "#c8c0b4",
      w: "#3e3630",
      E: "#1e140c",
      e: "#8a6a40",
      N: "#7a8a70",
      P: "#e8d8a0",
      p: "#d8c890",
      L: "#4a4a50",
      l: "#34343a",
      u: "#e2cfa4",
    },
    build(set) {
      gyps(set);
      longBill(set, "p");
    },
    signature: {
      // The bare neck reaches deep into a carcass without soiling the feathers.
      label: "reaches into the carcass",
      clip: (s) => {
        s.parts.reach = ["HH....", "wHH...", ".wHH..", "..wHH.", "...wHH"];
        SCRAP(s);
        return {
          loop: false,
          frames: [
            perch("head", 14, 1, 300),
            perch("head_down", 16, 5, 200, { dx: 1 }),
            perch("head_down", 19, 10, 260, {
              dx: 1,
              more: [["reach", 15, 8]],
            }),
            perch("head_down", 19, 11, 160, {
              dx: 1,
              dy: 1,
              more: [["reach", 15, 8]],
            }),
            perch("head_down", 19, 10, 160, {
              dx: 1,
              more: [["reach", 15, 8]],
            }),
            perch("head_down:blink", 19, 11, 140, {
              dx: 1,
              dy: 1,
              more: [["reach", 15, 8]],
            }),
            perch("head_down", 16, 5, 160, { more: [["scrap", 21, 12]] }),
            perch("head_up", 14, 0, 260, { more: [["scrap", 22, -2]] }),
            perch("head_up:blink", 14, -1, 160),
            perch("head", 14, 1, 800),
          ],
        };
      },
    },
  },
  {
    id: "gyps-tenuirostris",
    name: "Slender-billed vulture",
    latin: "Gyps tenuirostris",
    family: "old-world",
    iucn: "CR",
    length: [77, 103],
    span: [196, 258],
    tall: true,
    neck: [3, "w", "H", "u"],
    palette: {
      K: "#2e2418",
      B: "#8a7050",
      b: "#c8b08a",
      s: "#a89070",
      x: "#1a140e",
      H: "#2e2a2a",
      h: "#5a5450",
      w: "#141212",
      E: "#1a1210",
      e: "#6a5030",
      N: "#141212",
      P: "#2a2626",
      p: "#bfb6a6",
      L: "#3a3a3e",
      l: "#26262a",
      u: "#d8d2c4",
    },
    build(set) {
      rect(set, BODIES, [2, 5, 10, 12], { b: "u", s: "u" });
      gyps(set);
      longBill(set, "P");
    },
    signature: {
      // The long, thin bill probes where thick bills cannot reach.
      label: "probes with its bill",
      clip: () => ({
        loop: false,
        frames: [
          perch("head", 14, 1, 300),
          perch("head_down", 16, 6, 140, { dy: 1 }),
          perch("head_down", 17, 7, 100, { dy: 1 }),
          perch("head_down", 16, 6, 100, { dy: 1 }),
          perch("head_down", 17, 7, 100, { dy: 1 }),
          perch("head_down", 16, 6, 100, { dy: 1 }),
          perch("head_down:blink", 17, 7, 120, { dy: 1 }),
          perch("head", 14, 1, 700),
        ],
      }),
    },
  },
  {
    id: "gyps-bengalensis",
    name: "White-rumped vulture",
    latin: "Gyps bengalensis",
    family: "old-world",
    iucn: "CR",
    length: [75, 93],
    span: [192, 260],
    tall: true,
    neck: [3, "w", "H", "u"],
    palette: {
      K: "#0a0909",
      B: "#151416",
      b: "#232124",
      s: "#3c393d",
      x: "#0e0d0e",
      H: "#9a7a7c",
      h: "#b89a9a",
      w: "#5a4a4c",
      E: "#1e140c",
      e: "#c8a050",
      N: "#1a1a1a",
      P: "#b8bcc4",
      p: "#d8dce2",
      L: "#2a2a2c",
      l: "#1a1a1c",
      u: "#f2efe8",
      v: "#a8a8b0",
    },
    build(set) {
      rect(set, BODIES, [2, 5, 10, 12], { b: "u", s: "u" });
      gyps(set);
      flightInner(set, 3, 14, { s: "v", b: "u" });
    },
    signature: {
      // In flight, the white back and underwing coverts show against the black body.
      label: "takes off, showing white",
      clip: () => ({
        loop: false,
        frames: [
          perch("head", 14, 1, 300),
          perch("head", 14, 2, 160, { dy: 1 }),
          perch("head_up", 14, 0, 120, { dy: -2, more: [["wing_up", 0, -9]] }),
          air("fly_up", 140, -5),
          air("fly_down", 160, -6),
          air("glide", 600, -8),
          air("glide", 400, -7),
          air("fly_up", 140, -5),
          perch("head_up", 14, 0, 160, { more: [["wing_up", 0, -9]] }),
          perch("head", 14, 1, 160, { dy: 1, more: [["wing_up", 1, -6]] }),
          perch("head", 14, 1, 700),
        ],
      }),
    },
  },
  {
    id: "aegypius",
    name: "Cinereous vulture",
    latin: "Aegypius monachus",
    family: "old-world",
    iucn: "NT",
    length: [98, 120],
    span: [250, 310],
    tall: true,
    neck: [1, "H", "w", "g"],
    palette: {
      K: "#0d0a08",
      B: "#1f1915",
      b: "#2e2620",
      s: "#4a3e33",
      H: "#3e332c",
      h: "#5a4a40",
      w: "#9aa3c8",
      E: "#1a1210",
      e: "#8a6a40",
      N: "#8a6aa0",
      P: "#8a90b8",
      p: "#2a2522",
      L: "#b8bccc",
      l: "#8a8ea0",
      g: "#3d3128",
    },
    build(set) {
      for (const p of BODIES) set.parts[p] = patch(set.parts[p], RUFF, "g");
      set.clips.fly = soar(1, 3200);
      set.clips.idle = slow(set.clips.idle, 1.4);
    },
    signature: {
      // The biggest Old World vulture fluffs up and spreads its wings to keep rivals off.
      label: "looms",
      clip: () => ({
        loop: false,
        frames: [
          perch("head", 14, 1, 400),
          perch("head_hiss", 13, 1, 200, { body: "body_puff" }),
          perch("head_hiss", 13, 0, 700, {
            body: "body_puff",
            dy: -1,
            more: [["wing_up", 0, -9]],
          }),
          perch("head_hiss:blink", 13, 0, 140, {
            body: "body_puff",
            dy: -1,
            more: [["wing_up", 0, -9]],
          }),
          perch("head_hiss", 13, 1, 300, {
            body: "body_puff",
            more: [["wing_up", 1, -6]],
          }),
          perch("head", 14, 1, 700),
        ],
      }),
    },
  },
  {
    id: "torgos",
    name: "Lappet-faced vulture",
    latin: "Torgos tracheliotos",
    family: "old-world",
    iucn: "EN",
    length: [95, 115],
    span: [250, 290],
    tall: true,
    neck: [3, "w", "H", "g"],
    palette: {
      K: "#120d0a",
      B: "#2a1f18",
      b: "#3a2c22",
      s: "#5a4535",
      H: "#e88a8a",
      h: "#f2a8a0",
      w: "#c25a5a",
      f: "#d86a6a",
      E: "#1a1010",
      e: "#c8b090",
      N: "#8a5a40",
      P: "#8a7a60",
      p: "#5a4e3e",
      L: "#a8b0c0",
      l: "#7a8090",
      u: "#f2efe8",
      g: "#4a3a2e",
    },
    build(set) {
      under(set, "f", 2);
      chest(set, set.tall ? 17 : 12, set.tall ? 22 : 15, 3, "u");
      for (const p of BODIES) set.parts[p] = patch(set.parts[p], RUFF, "g");
      // The bully of the carcass shakes harder.
      set.clips.fail = {
        loop: true,
        frames: set.clips.fail.frames.map((f) => ({ ...f, dx: f.dx * 2 })),
      };
      set.clips.fly = soar(1, 3000);
    },
    signature: {
      label: "charges",
      clip: () => ({
        loop: false,
        frames: [
          perch("head", 14, 1, 300),
          perch("head_hiss", 14, 3, 160, { dy: 1 }),
          perch("head_hiss", 15, 3, 90, { dx: 2, more: [["wing_up", 1, -6]] }),
          perch("head_hiss", 16, 3, 90, {
            dx: 4,
            dy: -1,
            more: [["wing_up", 0, -8]],
          }),
          perch("head_hiss", 15, 2, 400, { dx: 4, more: [["wing_up", 1, -6]] }),
          perch("head", 14, 1, 200, { dx: 2 }),
          perch("head", 14, 1, 600),
        ],
      }),
    },
  },
  {
    id: "sarcogyps",
    name: "Red-headed vulture",
    latin: "Sarcogyps calvus",
    family: "old-world",
    iucn: "CR",
    length: [76, 86],
    span: [199, 260],
    neck: [2, "w", "H"],
    palette: {
      K: "#050404",
      B: "#100e0f",
      b: "#1a1718",
      s: "#3a3536",
      H: "#d8322e",
      h: "#e85a4a",
      w: "#a01e1e",
      f: "#c4282a",
      E: "#efe8d8",
      e: "#1a1010",
      N: "#4a2a2a",
      P: "#2a2222",
      p: "#4a3a3a",
      L: "#d8605a",
      l: "#a8403a",
      u: "#f2efe8",
    },
    build(set) {
      under(set, "f", 2);
      chest(set, 3, 5, 2, "u");
      chest(set, 12, 14, 3, "u");
      set.clips.fly = soar(2, 2200);
    },
    signature: {
      // After eating it shakes its head and the lappets swing.
      label: "shakes its head",
      clip: () => ({
        loop: false,
        frames: [
          perch("head", 14, 1, 400),
          perch("head", 13, 1, 70),
          perch("head", 15, 1, 70),
          perch("head", 13, 1, 70),
          perch("head", 15, 1, 70),
          perch("head", 14, 0, 90),
          perch("head:blink", 14, 1, 140),
          perch("head", 14, 1, 700),
        ],
      }),
    },
  },
  {
    id: "trigonoceps",
    name: "White-headed vulture",
    latin: "Trigonoceps occipitalis",
    family: "old-world",
    iucn: "CR",
    length: [72, 85],
    span: [207, 230],
    neck: [2, "w", "H"],
    palette: {
      K: "#0a0909",
      B: "#161415",
      b: "#232021",
      s: "#3e393a",
      H: "#f0e8e0",
      h: "#f8f6f2",
      w: "#e8a0a8",
      E: "#2a1a10",
      e: "#d8c8a0",
      N: "#7aa0d8",
      P: "#c8404a",
      p: "#e8a07a",
      L: "#e8a8a0",
      l: "#b87870",
      u: "#f2efe8",
    },
    build(set) {
      chest(set, 9, 14, 4, "u");
      // White secondaries: the female's (the male's are dark grey).
      flightInner(set, 3, 12, { s: "u", b: "u" });
      set.clips.fly = soar(2, 2000);
    },
    signature: {
      // Unlike most vultures it sometimes hunts live prey.
      label: "hunts",
      clip: () => ({
        loop: false,
        frames: [
          perch("head_down", 15, 3, 600),
          perch("head_down", 15, 4, 200, { dy: 1 }),
          perch("head_up", 14, 0, 120, {
            dy: -3,
            legs: "legs_step",
            more: [["wing_up", 0, -9]],
          }),
          perch("head_down", 16, 5, 120, {
            dx: 3,
            dy: -1,
            more: [["wing_up", 1, -6]],
          }),
          perch("head_down", 16, 7, 300, { dx: 3, dy: 1 }),
          perch("head", 14, 1, 300, { dx: 3 }),
          perch("head", 14, 1, 600),
        ],
      }),
    },
  },
];

export const sizeOf = (s: Species): Size =>
  s.size ?? sizeFrom(s.length, s.span);
